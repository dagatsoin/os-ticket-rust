//! Integration tests for TS-M4-H1 — canned-response CRUD + can_manage_premade gate.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test canned_crud
//! ```
//!
//! @implements FS-022 + BS-001 (admin OR can_manage_premade gate).

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

fn test_db_url() -> Option<String> { std::env::var("TEST_DATABASE_URL").ok() }
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
        if let Some(v) = first.strip_prefix(&format!("{name}=")) { return Some(v.to_string()); }
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
async fn send_json(router: &Router, method: &str, uri: &str, sess: &str, csrf: &str, body: Value) -> http::Response<Body> {
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

/// Build a multipart/form-data body from text fields + an optional file part.
fn multipart(fields: &[(&str, &str)], file: Option<(&str, &str, &[u8])>) -> (String, Vec<u8>) {
    let boundary = format!("----test{}", nonce());
    let mut body: Vec<u8> = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes());
        body.extend_from_slice(value.as_bytes());
        body.extend_from_slice(b"\r\n");
    }
    if let Some((field, filename, bytes)) = file {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(format!("Content-Disposition: form-data; name=\"{field}\"; filename=\"{filename}\"\r\n").as_bytes());
        body.extend_from_slice(b"Content-Type: text/plain\r\n\r\n");
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    (boundary, body)
}
async fn send_multipart(router: &Router, method: &str, uri: &str, sess: &str, csrf: &str,
    fields: &[(&str, &str)], file: Option<(&str, &str, &[u8])>) -> http::Response<Body> {
    let (boundary, body) = multipart(fields, file);
    router.clone().oneshot(Request::builder().method(method).uri(uri)
        .header(COOKIE, format!("ost_staff_sess={sess}; XSRF-TOKEN-STAFF={csrf}"))
        .header(CSRF_HEADER, csrf)
        .header("content-type", format!("multipart/form-data; boundary={boundary}"))
        .body(Body::from(body)).unwrap()).await.unwrap()
}
macro_rules! skip_if_no_db { ($n:expr) => {{ let Some(p)=seeded_pool().await else { eprintln!("skip {}", $n); return; }; p }}; }
fn nonce() -> u128 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() }

/// Mint a premade1 account whose group carries can_manage_premade.
async fn mint_premade(router: &Router) {
    // Fresh group + staff.
    let seed = json_body(dev(router, "/api/dev/seed-staff",
        json!({ "username": "premade1", "password": "Premade123!", "isadmin": false })).await).await;
    let group_id = seed["groupId"].as_i64().unwrap();
    dev(router, "/api/dev/set-group-perm",
        json!({ "groupId": group_id, "can_manage_premade": true })).await;
}

// --- AC-6 + AC-1: set-group-perm toggles flags; gate admits premade/admin ----

#[tokio::test]
async fn gate_admits_admin_and_premade_denies_agent() {
    let pool = skip_if_no_db!("gate_admits_admin_and_premade_denies_agent");
    let router = dev_app(pool);
    mint_premade(&router).await;

    let (admin, _) = login(&router, "admin", "Admin123!").await;
    let (premade, _) = login(&router, "premade1", "Premade123!").await;
    let (agent, _) = login(&router, "agent", "Agent123!").await;

    assert_eq!(get(&router, "/api/staff/canned-responses", &admin).await.status(), StatusCode::OK, "admin passes");
    assert_eq!(get(&router, "/api/staff/canned-responses", &premade).await.status(), StatusCode::OK, "premade1 passes");
    assert_eq!(get(&router, "/api/staff/canned-responses", &agent).await.status(), StatusCode::FORBIDDEN, "plain agent denied");
}

// --- AC-6: set-group-perm sets both delegated flags --------------------------

#[tokio::test]
async fn set_group_perm_toggles_delegated_flags() {
    let pool = skip_if_no_db!("set_group_perm_toggles_delegated_flags");
    let router = dev_app(pool.clone());
    let seed = json_body(dev(&router, "/api/dev/seed-staff",
        json!({ "username": format!("dg{}", nonce()), "password": "Pw123456!", "isadmin": false })).await).await;
    let group_id = seed["groupId"].as_i64().unwrap();
    let resp = dev(&router, "/api/dev/set-group-perm",
        json!({ "groupId": group_id, "can_manage_premade": true, "can_manage_faq": true })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let flags: (bool, bool) = sqlx::query_as("SELECT can_manage_premade, can_manage_faq FROM groups WHERE group_id = $1")
        .bind(group_id as i32).fetch_one(&pool).await.unwrap();
    assert_eq!(flags, (true, true), "both delegated flags set");
}

// --- AC-2: create validation (no title) --------------------------------------

#[tokio::test]
async fn create_requires_title() {
    let pool = skip_if_no_db!("create_requires_title");
    let router = dev_app(pool);
    mint_premade(&router).await;
    let (premade, csrf) = login(&router, "premade1", "Premade123!").await;

    let resp = send_multipart(&router, "POST", "/api/staff/canned-responses", &premade, &csrf,
        &[("response", "x")], None).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json_body(resp).await["error"]["fields"]["title"].is_string());
}

// --- AC-3: create with token body + attachment; edit retains via keep_file_ids

#[tokio::test]
async fn create_with_attachment_and_keep_on_edit() {
    let pool = skip_if_no_db!("create_with_attachment_and_keep_on_edit");
    let router = dev_app(pool);
    mint_premade(&router).await;
    let (premade, csrf) = login(&router, "premade1", "Premade123!").await;

    let title = format!("T {}", nonce());
    let resp = send_multipart(&router, "POST", "/api/staff/canned-responses", &premade, &csrf,
        &[("title", &title), ("response", "Hi %{ticket.name}")],
        Some(("attachment", "note.txt", b"hello world"))).await;
    assert!(resp.status() == StatusCode::OK || resp.status() == StatusCode::CREATED, "create ok");
    let id = json_body(resp).await["id"].as_i64().unwrap();

    // Detail shows the attachment linked.
    let detail = json_body(get(&router, &format!("/api/staff/canned-responses/{id}"), &premade).await).await;
    let atts = detail["attachments"].as_array().unwrap();
    assert_eq!(atts.len(), 1, "one attachment linked");
    let file_id = atts[0]["id"].as_i64().unwrap();
    assert_eq!(detail["response"], "Hi %{ticket.name}");

    // Edit keeping the existing attachment id → retained.
    let resp = send_multipart(&router, "PUT", &format!("/api/staff/canned-responses/{id}"), &premade, &csrf,
        &[("title", &title), ("response", "Hi again"), ("keep_file_ids", &file_id.to_string())], None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let detail = json_body(get(&router, &format!("/api/staff/canned-responses/{id}"), &premade).await).await;
    assert_eq!(detail["attachments"].as_array().unwrap().len(), 1, "attachment retained via keep_file_ids");

    // Edit omitting keep_file_ids → unlinked.
    let resp = send_multipart(&router, "PUT", &format!("/api/staff/canned-responses/{id}"), &premade, &csrf,
        &[("title", &title), ("response", "Hi no files")], None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let detail = json_body(get(&router, &format!("/api/staff/canned-responses/{id}"), &premade).await).await;
    assert_eq!(detail["attachments"].as_array().unwrap().len(), 0, "attachment unlinked when omitted");
}

// --- AC-5: dept-options readable under the premade gate -----------------------

#[tokio::test]
async fn dept_options_under_premade_gate() {
    let pool = skip_if_no_db!("dept_options_under_premade_gate");
    let router = dev_app(pool);
    mint_premade(&router).await;
    let (premade, _) = login(&router, "premade1", "Premade123!").await;

    let resp = get(&router, "/api/staff/canned-responses/dept-options", &premade).await;
    assert_eq!(resp.status(), StatusCode::OK, "readable without the admin gate");
    let arr = json_body(resp).await;
    let arr = arr.as_array().unwrap();
    assert!(!arr.is_empty());
    assert!(arr[0].get("id").is_some() && arr[0].get("name").is_some());
}

// --- AC-4: disabling removes it from the active consumption list --------------

#[tokio::test]
async fn disabling_removes_from_consumption() {
    let pool = skip_if_no_db!("disabling_removes_from_consumption");
    let router = dev_app(pool.clone());
    mint_premade(&router).await;
    let (premade, csrf) = login(&router, "premade1", "Premade123!").await;
    let (agent, _) = login(&router, "agent", "Agent123!").await;

    // Create an enabled dept-0 (all-dept) canned response.
    let title = format!("Consume {}", nonce());
    let id = json_body(send_multipart(&router, "POST", "/api/staff/canned-responses", &premade, &csrf,
        &[("title", &title), ("response", "body"), ("dept_id", "0"), ("isenabled", "true")], None).await)
        .await["id"].as_i64().unwrap();

    // Seed a ticket so the M2 consumption endpoint has a ticket to scope against.
    let seed = json_body(dev(&router, "/api/dev/seed-ticket", json!({})).await).await;
    let tnum = seed["ticketNumber"].as_i64().unwrap();
    let ticket_id: i64 = sqlx::query_scalar(r#"SELECT ticket_id FROM ticket WHERE "ticketID" = $1"#)
        .bind(tnum).fetch_one(&pool).await.unwrap();

    // Enabled → appears in the reply dropdown consumption list.
    let list = json_body(get(&router, &format!("/api/staff/tickets/{ticket_id}/canned"), &agent).await).await;
    assert!(list.as_array().unwrap().iter().any(|c| c["id"] == id), "enabled response offered");

    // Disable it → absent from the active consumption list.
    send_multipart(&router, "PUT", &format!("/api/staff/canned-responses/{id}"), &premade, &csrf,
        &[("title", &title), ("response", "body"), ("dept_id", "0"), ("isenabled", "false")], None).await;
    let list = json_body(get(&router, &format!("/api/staff/tickets/{ticket_id}/canned"), &agent).await).await;
    assert!(!list.as_array().unwrap().iter().any(|c| c["id"] == id), "disabled response no longer offered");
}

// --- delete + mass ------------------------------------------------------------

#[tokio::test]
async fn delete_and_mass() {
    let pool = skip_if_no_db!("delete_and_mass");
    let router = dev_app(pool);
    mint_premade(&router).await;
    let (premade, csrf) = login(&router, "premade1", "Premade123!").await;

    let mk = |t: String| async move { t };
    let t1 = mk(format!("D1 {}", nonce())).await;
    let id1 = json_body(send_multipart(&router, "POST", "/api/staff/canned-responses", &premade, &csrf,
        &[("title", &t1), ("response", "b")], None).await).await["id"].as_i64().unwrap();
    let t2 = format!("D2 {}", nonce());
    let id2 = json_body(send_multipart(&router, "POST", "/api/staff/canned-responses", &premade, &csrf,
        &[("title", &t2), ("response", "b")], None).await).await["id"].as_i64().unwrap();

    // mass disable both.
    let resp = send_json(&router, "POST", "/api/staff/canned-responses/mass", &premade, &csrf,
        json!({ "action": "disable", "ids": [id1, id2] })).await;
    assert_eq!(json_body(resp).await["affected"], 2);

    // delete id1.
    let resp = send_json(&router, "DELETE", &format!("/api/staff/canned-responses/{id1}"), &premade, &csrf, json!({})).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["ok"], true);
    assert_eq!(get(&router, &format!("/api/staff/canned-responses/{id1}"), &premade).await.status(), StatusCode::NOT_FOUND);
}
