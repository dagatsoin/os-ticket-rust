//! Integration tests for TS-M4-C1 — department CRUD + delete re-home + default
//! protection + group-access full-replace.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test departments
//! ```
//!
//! @implements FS-030.5/.6/.7: list/create/edit/delete-re-home.
//! @implements BS-030-01..08: validation, default protection, group-access sync.

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

/// Fetch a valid (email_id, tpl_id) pair from the form-options endpoint.
async fn valid_refs(router: &Router, sess: &str) -> (i64, i64) {
    let opts = json_body(get(router, "/api/staff/admin/departments/form-options", sess).await).await;
    let email_id = opts["email_accounts"][0]["id"].as_i64().expect("seeded email_account");
    let tpl_id = opts["template_groups"][0]["id"].as_i64().expect("seeded template_group");
    (email_id, tpl_id)
}

// --- AC-6 (+ list + gate): form-options shape; agent → 403 -------------------

#[tokio::test]
async fn form_options_and_gate() {
    let pool = skip_if_no_db!("form_options_and_gate");
    let router = dev_app(pool);
    let (admin, _) = login(&router, "admin", "Admin123!").await;

    let resp = get(&router, "/api/staff/admin/departments/form-options", &admin).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let opts = json_body(resp).await;
    for k in ["email_accounts", "template_groups", "sla", "staff", "groups"] {
        assert!(opts[k].is_array(), "form-options missing {k}");
    }
    // NET-NEW reads resolve: the seeded support@osticket.local + osTicket Default.
    assert!(opts["email_accounts"].as_array().unwrap().iter()
        .any(|e| e["email"] == "support@osticket.local"), "seeded email account surfaced");
    assert!(opts["template_groups"].as_array().unwrap().iter()
        .any(|t| t["name"] == "osTicket Default"), "seeded template group surfaced");

    // list is admin-only.
    let list = json_body(get(&router, "/api/staff/admin/departments", &admin).await).await;
    assert_eq!(list.as_array().unwrap().iter().filter(|d| d["is_default"] == true).count(), 1);

    let (agent, _) = login(&router, "agent", "Agent123!").await;
    assert_eq!(get(&router, "/api/staff/admin/departments/form-options", &agent).await.status(), StatusCode::FORBIDDEN);
    assert_eq!(get(&router, "/api/staff/admin/departments", &agent).await.status(), StatusCode::FORBIDDEN);
}

// --- AC-1: email + template required -----------------------------------------

#[tokio::test]
async fn email_and_template_required() {
    let pool = skip_if_no_db!("email_and_template_required");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    // Valid name, no email.
    let resp = send(&router, "POST", "/api/staff/admin/departments", &admin, &csrf,
        json!({ "name": format!("Dept {}", nonce()) })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let err = json_body(resp).await;
    assert_eq!(err["error"]["fields"]["emailId"], "Email selection required");

    // Email set, no template.
    let (email_id, _) = valid_refs(&router, &admin).await;
    let resp = send(&router, "POST", "/api/staff/admin/departments", &admin, &csrf,
        json!({ "name": format!("Dept {}", nonce()), "emailId": email_id })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let err = json_body(resp).await;
    assert_eq!(err["error"]["fields"]["tplId"], "Template selection required");
}

// --- AC-2: default cannot be private/deleted ---------------------------------

#[tokio::test]
async fn default_cannot_be_private_or_deleted() {
    let pool = skip_if_no_db!("default_cannot_be_private_or_deleted");
    let router = dev_app(pool.clone());
    dev(&router, "/api/dev/reset-departments", json!({})).await;
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let (email_id, tpl_id) = valid_refs(&router, &admin).await;

    let list = json_body(get(&router, "/api/staff/admin/departments", &admin).await).await;
    let default_id = list.as_array().unwrap().iter().find(|d| d["is_default"] == true).unwrap()["id"].as_i64().unwrap();

    // PUT ispublic=false → 422.
    let resp = send(&router, "PUT", &format!("/api/staff/admin/departments/{default_id}"), &admin, &csrf,
        json!({ "name": "Support", "ispublic": false, "emailId": email_id, "tplId": tpl_id })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json_body(resp).await["error"]["message"].as_str().unwrap().contains("cannot be private"));

    // DELETE default → refused.
    let resp = send(&router, "DELETE", &format!("/api/staff/admin/departments/{default_id}"), &admin, &csrf, json!({})).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let still: Option<i32> = sqlx::query_scalar("SELECT dept_id FROM department WHERE dept_id = $1")
        .bind(default_id as i32).fetch_optional(&pool).await.unwrap();
    assert!(still.is_some(), "default dept not deleted");
}

// --- AC-3: delete refused with home staff; else re-homes tickets + topics -----

#[tokio::test]
async fn delete_home_staff_guard_then_rehome() {
    let pool = skip_if_no_db!("delete_home_staff_guard_then_rehome");
    let router = dev_app(pool.clone());
    dev(&router, "/api/dev/reset-departments", json!({})).await;
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let (email_id, tpl_id) = valid_refs(&router, &admin).await;

    let list = json_body(get(&router, "/api/staff/admin/departments", &admin).await).await;
    let default_id = list.as_array().unwrap().iter().find(|d| d["is_default"] == true).unwrap()["id"].as_i64().unwrap() as i32;

    // Create a non-default dept.
    let dept_id = json_body(send(&router, "POST", "/api/staff/admin/departments", &admin, &csrf,
        json!({ "name": format!("Ops {}", nonce()), "emailId": email_id, "tplId": tpl_id })).await).await["id"].as_i64().unwrap() as i32;

    // Seed a staff homed to it, a ticket homed to it, a help topic homed to it.
    let group_id: i32 = sqlx::query_scalar("SELECT group_id FROM groups LIMIT 1").fetch_one(&pool).await.unwrap();
    let staff_id: i32 = sqlx::query_scalar(
        "INSERT INTO staff (group_id, dept_id, username, passwd) VALUES ($1, $2, $3, 'x') RETURNING staff_id")
        .bind(group_id).bind(dept_id).bind(format!("homed{}", nonce())).fetch_one(&pool).await.unwrap();
    let ticket_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO ticket ("ticketID", dept_id, email, subject) VALUES ($1, $2, $3, 'x') RETURNING ticket_id"#)
        .bind(700000 + (nonce() % 200000) as i64).bind(dept_id).bind(format!("d{}@ex.com", nonce()))
        .fetch_one(&pool).await.unwrap();
    let topic_id: i32 = sqlx::query_scalar(
        "INSERT INTO help_topic (topic, dept_id) VALUES ($1, $2) RETURNING topic_id")
        .bind(format!("Topic {}", nonce())).bind(dept_id).fetch_one(&pool).await.unwrap();

    // DELETE while home staff remain → refused.
    let resp = send(&router, "DELETE", &format!("/api/staff/admin/departments/{dept_id}"), &admin, &csrf, json!({})).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json_body(resp).await["error"]["message"].as_str().unwrap().contains("staff members"));

    // Move the staff away, then DELETE → 200, tickets + topics re-homed to default.
    sqlx::query("UPDATE staff SET dept_id = $1 WHERE staff_id = $2").bind(default_id).bind(staff_id).execute(&pool).await.unwrap();
    let resp = send(&router, "DELETE", &format!("/api/staff/admin/departments/{dept_id}"), &admin, &csrf, json!({})).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let t_dept: i32 = sqlx::query_scalar("SELECT dept_id FROM ticket WHERE ticket_id = $1").bind(ticket_id).fetch_one(&pool).await.unwrap();
    assert_eq!(t_dept, default_id, "ticket re-homed to default");
    let h_dept: i32 = sqlx::query_scalar("SELECT dept_id FROM help_topic WHERE topic_id = $1").bind(topic_id).fetch_one(&pool).await.unwrap();
    assert_eq!(h_dept, default_id, "topic re-homed to default");
    let gone: Option<i32> = sqlx::query_scalar("SELECT dept_id FROM department WHERE dept_id = $1").bind(dept_id).fetch_optional(&pool).await.unwrap();
    assert!(gone.is_none(), "dept deleted");
}

// --- AC-4: group-access full-replace -----------------------------------------

#[tokio::test]
async fn group_access_full_replace() {
    let pool = skip_if_no_db!("group_access_full_replace");
    let router = dev_app(pool.clone());
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let (email_id, tpl_id) = valid_refs(&router, &admin).await;

    let groups: Vec<i32> = sqlx::query_scalar("SELECT group_id FROM groups ORDER BY group_id LIMIT 2").fetch_all(&pool).await.unwrap();
    assert!(groups.len() >= 2, "need two seeded groups");
    let (g1, g2) = (groups[0], groups[1]);

    let dept_id = json_body(send(&router, "POST", "/api/staff/admin/departments", &admin, &csrf,
        json!({ "name": format!("GA {}", nonce()), "emailId": email_id, "tplId": tpl_id, "groupIds": [g1] })).await).await["id"].as_i64().unwrap();

    // Initial set = [g1].
    let d = json_body(get(&router, &format!("/api/staff/admin/departments/{dept_id}"), &admin).await).await;
    assert_eq!(d["group_ids"], json!([g1]));

    // PUT a different set → exactly [g2].
    let resp = send(&router, "PUT", &format!("/api/staff/admin/departments/{dept_id}"), &admin, &csrf,
        json!({ "name": format!("GA {}", nonce()), "emailId": email_id, "tplId": tpl_id, "groupIds": [g2] })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let d = json_body(get(&router, &format!("/api/staff/admin/departments/{dept_id}"), &admin).await).await;
    assert_eq!(d["group_ids"], json!([g2]), "group access fully replaced");
}

// --- AC-5: no-change re-save reports success ---------------------------------

#[tokio::test]
async fn no_change_resave_succeeds() {
    let pool = skip_if_no_db!("no_change_resave_succeeds");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let (email_id, tpl_id) = valid_refs(&router, &admin).await;

    let name = format!("Stable {}", nonce());
    let dept_id = json_body(send(&router, "POST", "/api/staff/admin/departments", &admin, &csrf,
        json!({ "name": name, "emailId": email_id, "tplId": tpl_id })).await).await["id"].as_i64().unwrap();

    // Re-submit the identical payload → 200, not "unable to update".
    let resp = send(&router, "PUT", &format!("/api/staff/admin/departments/{dept_id}"), &admin, &csrf,
        json!({ "name": name, "emailId": email_id, "tplId": tpl_id })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["id"].as_i64().unwrap(), dept_id);
}

// --- duplicate name → 422 keyed on name --------------------------------------

#[tokio::test]
async fn duplicate_name_rejected() {
    let pool = skip_if_no_db!("duplicate_name_rejected");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let (email_id, tpl_id) = valid_refs(&router, &admin).await;
    let resp = send(&router, "POST", "/api/staff/admin/departments", &admin, &csrf,
        json!({ "name": "Support", "emailId": email_id, "tplId": tpl_id })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json_body(resp).await["error"]["fields"]["name"].is_string());
}
