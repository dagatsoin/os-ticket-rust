//! Integration tests for TS-M4-E1 — FAQ-category CRUD + `can_manage_faq` gate.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test faq_categories
//! ```
//!
//! @implements FS-032.13/.14/.15/.16.
//! @implements KL-032.11: missing id returns 404, not a hollow object.

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

// --- AC-1: gate allows can_manage_faq + admin, denies plain agent -------------

#[tokio::test]
async fn gate_admin_and_flag_pass_agent_denied() {
    let pool = skip_if_no_db!("gate_admin_and_flag_pass_agent_denied");
    let router = dev_app(pool);

    // Create faqmgr with a can_manage_faq group.
    let un = format!("faqmgr{}", nonce());
    let seeded = json_body(dev(&router, "/api/dev/seed-staff", json!({ "username": un, "password": "Faqmgr123!" })).await).await;
    let group_id = seeded["groupId"].as_i64().unwrap();
    // set-group-perm deserializes camelCase (`groupId`); the flag accepts the snake alias.
    let sp = dev(&router, "/api/dev/set-group-perm", json!({ "groupId": group_id, "can_manage_faq": true })).await;
    assert_eq!(sp.status(), StatusCode::OK, "set-group-perm applied");

    let (faqmgr, _) = login(&router, &un, "Faqmgr123!").await;
    assert_eq!(get(&router, "/api/staff/faq-categories", &faqmgr).await.status(), StatusCode::OK, "faqmgr passes");

    let (agent, _) = login(&router, "agent", "Agent123!").await;
    assert_eq!(get(&router, "/api/staff/faq-categories", &agent).await.status(), StatusCode::FORBIDDEN, "plain agent denied");

    let (admin, _) = login(&router, "admin", "Admin123!").await;
    assert_eq!(get(&router, "/api/staff/faq-categories", &admin).await.status(), StatusCode::OK, "admin passes");
}

// --- AC-2: create validation --------------------------------------------------

#[tokio::test]
async fn create_validation() {
    let pool = skip_if_no_db!("create_validation");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    let resp = send(&router, "POST", "/api/staff/faq-categories", &admin, &csrf, json!({ "ispublic": true })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json_body(resp).await["error"]["fields"]["name"].is_string());
}

// --- AC-3: plain delete -------------------------------------------------------

#[tokio::test]
async fn plain_delete() {
    let pool = skip_if_no_db!("plain_delete");
    let router = dev_app(pool.clone());
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    let id = json_body(send(&router, "POST", "/api/staff/faq-categories", &admin, &csrf,
        json!({ "name": format!("Cat {}", nonce()), "ispublic": true, "description": "d" })).await).await["id"].as_i64().unwrap();

    let resp = send(&router, "DELETE", &format!("/api/staff/faq-categories/{id}"), &admin, &csrf, json!({})).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let gone: Option<i32> = sqlx::query_scalar("SELECT id FROM faq_category WHERE id = $1").bind(id as i32).fetch_optional(&pool).await.unwrap();
    assert!(gone.is_none(), "category deleted");
}

// --- AC-4: missing id → 404 (GET + PUT) --------------------------------------

#[tokio::test]
async fn missing_id_404() {
    let pool = skip_if_no_db!("missing_id_404");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    assert_eq!(get(&router, "/api/staff/faq-categories/999999", &admin).await.status(), StatusCode::NOT_FOUND);
    let resp = send(&router, "PUT", "/api/staff/faq-categories/999999", &admin, &csrf, json!({ "name": "X", "ispublic": true })).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// --- CRUD round-trip + mass makeprivate ---------------------------------------

#[tokio::test]
async fn crud_and_mass() {
    let pool = skip_if_no_db!("crud_and_mass");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    dev(&router, "/api/dev/reset-faq-categories", json!({})).await;

    let id = json_body(send(&router, "POST", "/api/staff/faq-categories", &admin, &csrf,
        json!({ "name": format!("General {}", nonce()), "ispublic": true, "notes": "n" })).await).await["id"].as_i64().unwrap();

    // edit.
    let resp = send(&router, "PUT", &format!("/api/staff/faq-categories/{id}"), &admin, &csrf,
        json!({ "name": format!("Renamed {}", nonce()), "ispublic": true })).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // mass makeprivate.
    let resp = send(&router, "POST", "/api/staff/faq-categories/mass", &admin, &csrf,
        json!({ "action": "makeprivate", "ids": [id] })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["affected"], 1);
    let cat = json_body(get(&router, &format!("/api/staff/faq-categories/{id}"), &admin).await).await;
    assert_eq!(cat["ispublic"], false, "made internal");
}
