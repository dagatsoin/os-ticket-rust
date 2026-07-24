//! Integration tests for TS-M4-D1 — SLA CRUD + deletion re-homing + priorities.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test sla
//! ```
//!
//! @implements FS-032.9/.10/.12: list/create/edit/mass/delete-re-home.
//! @implements BS-032.13: read-only priority set.

use api::{app, AppEnv, AppState};
use axum::body::Body;
use axum::Router;
use http::header::{COOKIE, SET_COOKIE};
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::postgres::PgPool;
use tower::ServiceExt;

const ORIGIN: &str = "http://localhost:3702";
const CSRF_HEADER: &str = "x-csrftoken";

fn test_db_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}
async fn seeded_pool() -> Option<PgPool> {
    let url = test_db_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    tools::seed(&pool).await.expect("seed");
    Some(pool)
}
fn dev_app(pool: PgPool) -> Router {
    app(AppState::with_pool(pool).with_app_env(AppEnv::Development), ORIGIN)
}
fn set_cookie_value(resp: &http::Response<Body>, name: &str) -> Option<String> {
    for hv in resp.headers().get_all(SET_COOKIE) {
        let s = hv.to_str().ok()?;
        let first = s.split(';').next().unwrap_or("");
        if let Some(v) = first.strip_prefix(&format!("{name}=")) {
            return Some(v.to_string());
        }
    }
    None
}
async fn json_body(resp: http::Response<Body>) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() }
}
async fn login(router: &Router, user: &str, pass: &str) -> (String, String) {
    let resp = router.clone().oneshot(Request::builder().method("POST").uri("/api/staff/login")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "username": user, "password": pass }).to_string())).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "login {user}");
    (set_cookie_value(&resp, "ost_staff_sess").unwrap(), set_cookie_value(&resp, "XSRF-TOKEN-STAFF").unwrap())
}
async fn get(router: &Router, uri: &str, sess: &str) -> http::Response<Body> {
    router.clone().oneshot(Request::builder().uri(uri)
        .header(COOKIE, format!("ost_staff_sess={sess}")).body(Body::empty()).unwrap()).await.unwrap()
}
async fn send(router: &Router, method: &str, uri: &str, sess: &str, csrf: &str, body: Value) -> http::Response<Body> {
    router.clone().oneshot(Request::builder().method(method).uri(uri)
        .header(COOKIE, format!("ost_staff_sess={sess}; XSRF-TOKEN-STAFF={csrf}"))
        .header(CSRF_HEADER, csrf).header("content-type", "application/json")
        .body(Body::from(body.to_string())).unwrap()).await.unwrap()
}
async fn dev(router: &Router, uri: &str, body: Value) -> http::Response<Body> {
    router.clone().oneshot(Request::builder().method("POST").uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string())).unwrap()).await.unwrap()
}
macro_rules! skip_if_no_db { ($n:expr) => {{ let Some(p)=seeded_pool().await else { eprintln!("skip {}", $n); return; }; p }}; }
fn nonce() -> u128 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() }

// --- AC-1: list + create; agent → 403 ----------------------------------------

#[tokio::test]
async fn list_create_and_gate_non_admin() {
    let pool = skip_if_no_db!("list_create_and_gate_non_admin");
    let router = dev_app(pool);
    dev(&router, "/api/dev/reset-sla", json!({})).await;
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    let resp = get(&router, "/api/staff/admin/sla", &admin).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let list = json_body(resp).await;
    let arr = list.as_array().unwrap();
    assert!(arr.iter().any(|s| s["name"] == "Default SLA"), "seeded plans listed");
    // exactly one is_default = the default_sla_id one.
    assert_eq!(arr.iter().filter(|s| s["is_default"] == true).count(), 1);

    let name = format!("Gold {}", nonce());
    let resp = send(&router, "POST", "/api/staff/admin/sla", &admin, &csrf,
        json!({ "name": name, "gracePeriod": 24, "isactive": true })).await;
    assert!(resp.status() == StatusCode::OK || resp.status() == StatusCode::CREATED);
    let created = json_body(resp).await;
    assert!(created["id"].as_i64().is_some());

    // agent (non-admin) → 403.
    let (agent, _) = login(&router, "agent", "Agent123!").await;
    assert_eq!(get(&router, "/api/staff/admin/sla", &agent).await.status(), StatusCode::FORBIDDEN);
}

// --- AC-2: validation (name + grace period) ----------------------------------

#[tokio::test]
async fn create_validation() {
    let pool = skip_if_no_db!("create_validation");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    let resp = send(&router, "POST", "/api/staff/admin/sla", &admin, &csrf, json!({})).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let err = json_body(resp).await;
    assert!(err["error"]["fields"]["name"].is_string());
    assert!(err["error"]["fields"]["grace_period"].is_string());
    // KL-032.4: SLA-appropriate copy, NOT the API-key artifact text.
    let name_msg = err["error"]["fields"]["name"].as_str().unwrap().to_lowercase();
    assert!(!name_msg.contains("api"), "not the api-key copy-paste artifact");
}

// --- AC-2b: duplicate name → 422 keyed on name (sla_name_key 23505) -----------

#[tokio::test]
async fn duplicate_name_rejected() {
    let pool = skip_if_no_db!("duplicate_name_rejected");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let resp = send(&router, "POST", "/api/staff/admin/sla", &admin, &csrf,
        json!({ "name": "Default SLA", "gracePeriod": 10 })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json_body(resp).await["error"]["fields"]["name"].is_string());
}

// --- AC-3: default protected; others re-home dependents ----------------------

#[tokio::test]
async fn default_protected_others_rehome() {
    let pool = skip_if_no_db!("default_protected_others_rehome");
    let router = dev_app(pool.clone());
    dev(&router, "/api/dev/reset-sla", json!({})).await;
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    let list = json_body(get(&router, "/api/staff/admin/sla", &admin).await).await;
    let arr = list.as_array().unwrap();
    let default_id = arr.iter().find(|s| s["is_default"] == true).unwrap()["id"].as_i64().unwrap();
    // A non-default seeded plan (e.g. "Urgent").
    let victim = arr.iter().find(|s| s["is_default"] != true).unwrap()["id"].as_i64().unwrap();

    // Bind the victim plan to a fresh department + a ticket.
    let dept_id: i32 = sqlx::query_scalar("INSERT INTO department (dept_name, sla_id) VALUES ($1, $2) RETURNING dept_id")
        .bind(format!("SLA Dept {}", nonce())).bind(victim as i32)
        .fetch_one(&pool).await.unwrap();
    let ticket_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO ticket ("ticketID", dept_id, sla_id, email, subject) VALUES ($1, $2, $3, $4, 'x')
           RETURNING ticket_id"#)
        .bind(600000 + (nonce() % 300000) as i64).bind(dept_id).bind(victim as i32)
        .bind(format!("sla{}@ex.com", nonce()))
        .fetch_one(&pool).await.unwrap();

    // DELETE the default → refused; row still present.
    let resp = send(&router, "DELETE", &format!("/api/staff/admin/sla/{default_id}"), &admin, &csrf, json!({})).await;
    assert!(resp.status() == StatusCode::UNPROCESSABLE_ENTITY || resp.status() == StatusCode::CONFLICT);
    let still: Option<i32> = sqlx::query_scalar("SELECT id FROM sla WHERE id = $1").bind(default_id as i32)
        .fetch_optional(&pool).await.unwrap();
    assert!(still.is_some(), "default plan not deleted");

    // DELETE the victim → 200; dependents re-homed.
    let resp = send(&router, "DELETE", &format!("/api/staff/admin/sla/{victim}"), &admin, &csrf, json!({})).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let dept_sla: Option<i32> = sqlx::query_scalar("SELECT sla_id FROM department WHERE dept_id = $1").bind(dept_id)
        .fetch_one(&pool).await.unwrap();
    assert_eq!(dept_sla, None, "dept sla_id re-homed to NULL (not 0)");
    let ticket_sla: Option<i32> = sqlx::query_scalar("SELECT sla_id FROM ticket WHERE ticket_id = $1").bind(ticket_id)
        .fetch_one(&pool).await.unwrap();
    assert_eq!(ticket_sla, Some(default_id as i32), "ticket re-homed to default");
}

// --- AC-4: priorities read set, ranked by urgency DESC -----------------------

#[tokio::test]
async fn priorities_ranked_readonly() {
    let pool = skip_if_no_db!("priorities_ranked_readonly");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    let resp = get(&router, "/api/staff/admin/priorities", &admin).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let arr = json_body(resp).await;
    let arr = arr.as_array().unwrap();
    let names: Vec<&str> = arr.iter().map(|p| p["priority_desc"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["Low", "Normal", "High", "Emergency"], "ranked by urgency DESC");

    // No mutation route exists.
    let resp = send(&router, "POST", "/api/staff/admin/priorities", &admin, &csrf, json!({ "priority": "x" })).await;
    assert!(resp.status() == StatusCode::NOT_FOUND || resp.status() == StatusCode::METHOD_NOT_ALLOWED);
}

// --- mass activate/disable ---------------------------------------------------

#[tokio::test]
async fn mass_activate_disable() {
    let pool = skip_if_no_db!("mass_activate_disable");
    let router = dev_app(pool);
    dev(&router, "/api/dev/reset-sla", json!({})).await;
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    let name = format!("Mass {}", nonce());
    let id = json_body(send(&router, "POST", "/api/staff/admin/sla", &admin, &csrf,
        json!({ "name": name, "gracePeriod": 8 })).await).await["id"].as_i64().unwrap();

    let resp = send(&router, "POST", "/api/staff/admin/sla/mass", &admin, &csrf,
        json!({ "action": "disable", "ids": [id] })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["affected"], 1);
}
