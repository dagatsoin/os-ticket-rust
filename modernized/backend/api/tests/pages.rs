//! Integration tests for TS-M4-F1 — page CRUD + in-use guards + content/config.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test pages
//! ```
//!
//! @implements FS-033.9/.10/.13/.14/.15/.16 + BS-033.9 in-use guards.

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
async fn get_anon(router: &Router, uri: &str) -> http::Response<Body> {
    router.clone().oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap()).await.unwrap()
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

// --- AC-1: create validation + uniqueness + bad type -------------------------

#[tokio::test]
async fn create_validation_and_uniqueness() {
    let pool = skip_if_no_db!("create_validation_and_uniqueness");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    // No name → 422 name.
    let resp = send(&router, "POST", "/api/staff/admin/pages", &admin, &csrf,
        json!({ "type": "landing", "body": "x" })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json_body(resp).await["error"]["fields"]["name"].is_string());

    // Create then dup → 422.
    let name = format!("Dup {}", nonce());
    let ok = send(&router, "POST", "/api/staff/admin/pages", &admin, &csrf,
        json!({ "name": name, "type": "other", "body": "x" })).await;
    assert!(ok.status() == StatusCode::OK || ok.status() == StatusCode::CREATED);
    let dupe = send(&router, "POST", "/api/staff/admin/pages", &admin, &csrf,
        json!({ "name": name, "type": "other", "body": "x" })).await;
    assert_eq!(dupe.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json_body(dupe).await["error"]["fields"]["name"].is_string());

    // Bad type → 422.
    let bad = send(&router, "POST", "/api/staff/admin/pages", &admin, &csrf,
        json!({ "name": format!("Bad {}", nonce()), "type": "bogus", "body": "x" })).await;
    assert_eq!(bad.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json_body(bad).await["error"]["fields"]["type"].is_string());
}

// --- AC-2: bound page in-use; delete AND disable refused ---------------------

#[tokio::test]
async fn in_use_page_delete_and_disable_refused() {
    let pool = skip_if_no_db!("in_use_page_delete_and_disable_refused");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    // Seed a landing page and bind it as landing_page_id.
    let id = json_body(dev(&router, "/api/dev/seed-page",
        json!({ "name": format!("Land {}", nonce()), "type": "landing" })).await).await["id"].as_i64().unwrap();
    dev(&router, "/api/dev/seed-config", json!({ "landing_page_id": id.to_string() })).await;

    // in_use should now be true in the list.
    let list = json_body(get(&router, "/api/staff/admin/pages?per_page=200", &admin).await).await;
    let row = list["items"].as_array().unwrap().iter().find(|p| p["id"] == id).unwrap();
    assert_eq!(row["in_use"], true);

    // DELETE via mass → refused (0 deleted).
    let resp = send(&router, "POST", "/api/staff/admin/pages/mass", &admin, &csrf,
        json!({ "action": "delete", "ids": [id] })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["affected"], 0);

    // Disable via PUT → refused with the pinned message.
    let resp = send(&router, "PUT", &format!("/api/staff/admin/pages/{id}"), &admin, &csrf,
        json!({ "name": row["name"], "type": "landing", "body": "x", "isactive": false })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let msg = json_body(resp).await["error"]["message"].as_str().unwrap().to_string();
    assert!(msg.contains("in-use CANNOT be disabled"), "pinned disable message");
}

// --- AC-3: bulk enable/disable/delete of unbound pages + topic ref reset ------

#[tokio::test]
async fn bulk_ops_and_topic_ref_reset() {
    let pool = skip_if_no_db!("bulk_ops_and_topic_ref_reset");
    let router = dev_app(pool.clone());
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    let p1 = json_body(dev(&router, "/api/dev/seed-page",
        json!({ "name": format!("Ub1 {}", nonce()), "type": "other" })).await).await["id"].as_i64().unwrap();
    let p2 = json_body(dev(&router, "/api/dev/seed-page",
        json!({ "name": format!("Ub2 {}", nonce()), "type": "other" })).await).await["id"].as_i64().unwrap();

    // Disable both → OK, both disabled.
    let resp = send(&router, "POST", "/api/staff/admin/pages/mass", &admin, &csrf,
        json!({ "action": "disable", "ids": [p1, p2] })).await;
    assert_eq!(json_body(resp).await["affected"], 2);
    // Enable both.
    let resp = send(&router, "POST", "/api/staff/admin/pages/mass", &admin, &csrf,
        json!({ "action": "enable", "ids": [p1, p2] })).await;
    assert_eq!(json_body(resp).await["affected"], 2);

    // Point a help topic at p1 (loose int, not FK). Per BS-033.9 that makes p1
    // in-use, so a mass delete of [p1, p2] deletes only p2 (partial), refusing p1.
    sqlx::query("UPDATE help_topic SET page_id = $1 WHERE topic_id = (SELECT topic_id FROM help_topic ORDER BY topic_id LIMIT 1)")
        .bind(p1 as i32).execute(&pool).await.unwrap();
    let resp = send(&router, "POST", "/api/staff/admin/pages/mass", &admin, &csrf,
        json!({ "action": "delete", "ids": [p1, p2] })).await;
    assert_eq!(json_body(resp).await["affected"], 1, "only the unbound page (p2) deleted; topic-referenced p1 refused");
    let p1_still: Option<i32> = sqlx::query_scalar("SELECT id FROM page WHERE id = $1").bind(p1 as i32)
        .fetch_optional(&pool).await.unwrap();
    assert!(p1_still.is_some(), "topic-referenced page not deleted");

    // Now unbind the topic and delete p1 → permitted; the in-txn safety resets
    // any lingering help_topic.page_id back to 0.
    sqlx::query("UPDATE help_topic SET page_id = 0 WHERE page_id = $1").bind(p1 as i32).execute(&pool).await.unwrap();
    let resp = send(&router, "POST", "/api/staff/admin/pages/mass", &admin, &csrf,
        json!({ "action": "delete", "ids": [p1] })).await;
    assert_eq!(json_body(resp).await["affected"], 1, "unbound page now deletes");
    let leftover: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM help_topic WHERE page_id = $1").bind(p1 as i32)
        .fetch_one(&pool).await.unwrap();
    assert_eq!(leftover, 0, "no dangling topic page_id after delete");
}

// --- AC-4: content/config read endpoints (admin-gated) -----------------------

#[tokio::test]
async fn content_and_config_admin_gated() {
    let pool = skip_if_no_db!("content_and_config_admin_gated");
    let router = dev_app(pool);
    let (admin, _) = login(&router, "admin", "Admin123!").await;
    let (agent, _) = login(&router, "agent", "Agent123!").await;

    let resp = get(&router, "/api/staff/admin/content/ticket_variables", &admin).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let vars = json_body(resp).await;
    assert!(vars["base"].as_array().unwrap().iter().any(|v| v["token"] == "ticket.number"));
    assert_eq!(get(&router, "/api/staff/admin/content/ticket_variables", &agent).await.status(), StatusCode::FORBIDDEN);

    let resp = get(&router, "/api/config/scp", &admin).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let scp = json_body(resp).await;
    assert!(scp["date_format"].is_string() && scp["max_file_uploads"].is_string());
    assert_eq!(get(&router, "/api/config/scp", &agent).await.status(), StatusCode::FORBIDDEN);
}

// --- AC-5: public `other` page by slug (unauth) ------------------------------

#[tokio::test]
async fn public_page_by_slug() {
    let pool = skip_if_no_db!("public_page_by_slug");
    let router = dev_app(pool);
    let name = format!("Terms Of Service {}", nonce());
    dev(&router, "/api/dev/seed-page", json!({ "name": name, "type": "other", "body": "<p>TOS</p>" })).await;

    // slugify the seeded name the same way the backend does.
    let slug: String = {
        let mut out = String::new();
        let mut dash = true;
        for ch in name.chars() {
            if ch.is_ascii_alphanumeric() { out.push(ch.to_ascii_lowercase()); dash = false; }
            else if !dash { out.push('-'); dash = true; }
        }
        while out.ends_with('-') { out.pop(); }
        out
    };

    let resp = get_anon(&router, &format!("/api/pages/{slug}")).await;
    assert_eq!(resp.status(), StatusCode::OK, "public page served with no session");
    let page = json_body(resp).await;
    assert!(page["body"].as_str().unwrap().contains("TOS"));

    // Unknown slug → 404.
    assert_eq!(get_anon(&router, "/api/pages/no-such-page-xyz").await.status(), StatusCode::NOT_FOUND);

    // A non-other type by slug → 404.
    let land = format!("Landing X {}", nonce());
    dev(&router, "/api/dev/seed-page", json!({ "name": land, "type": "landing", "body": "z" })).await;
    let land_slug = land.to_lowercase().replace(' ', "-");
    assert_eq!(get_anon(&router, &format!("/api/pages/{land_slug}")).await.status(), StatusCode::NOT_FOUND);
}
