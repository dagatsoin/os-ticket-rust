//! Integration tests for TS-M4-B3 — admin group CRUD + 11-flag set + dept sync.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test admin_groups
//! ```
//!
//! @implements FS-031.7/.8: list/create/edit/mass.
//! @implements BS-031-019/020/021/022/023: name, 11 flags, dept sync, delete/self guards.

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
macro_rules! skip_if_no_db { ($n:expr) => {{ let Some(p)=seeded_pool().await else { eprintln!("skip {}", $n); return; }; p }}; }
fn nonce() -> u128 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() }

fn all_flags_on() -> Value {
    json!({
        "can_create_tickets": true, "can_edit_tickets": true, "can_post_reply": true,
        "can_close_tickets": true, "can_assign_tickets": true, "can_transfer_tickets": true,
        "can_delete_tickets": true, "can_manage_faq": true, "can_manage_premade": true,
        "can_ban_emails": true, "can_view_staff_stats": true
    })
}

// --- AC-1: list returns counts; agent → 403 ----------------------------------

#[tokio::test]
async fn list_returns_counts_and_gates_non_admin() {
    let pool = skip_if_no_db!("list_returns_counts_and_gates_non_admin");
    let router = dev_app(pool);
    let (admin, _) = login(&router, "admin", "Admin123!").await;

    let resp = get(&router, "/api/staff/admin/groups", &admin).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let groups = json_body(resp).await;
    let arr = groups.as_array().unwrap();
    assert!(!arr.is_empty());
    // The seeded "M1 Agents" group has ≥1 member + ≥1 dept-access row.
    let agents = arr.iter().find(|g| g["name"] == "M1 Agents").expect("M1 Agents present");
    assert!(agents["member_count"].as_i64().unwrap() >= 1);
    assert!(agents["dept_count"].as_i64().unwrap() >= 1);

    let (agent, _) = login(&router, "agent", "Agent123!").await;
    assert_eq!(get(&router, "/api/staff/admin/groups", &agent).await.status(), StatusCode::FORBIDDEN);
}

// --- AC-2: name validation ---------------------------------------------------

#[tokio::test]
async fn name_validation() {
    let pool = skip_if_no_db!("name_validation");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    let resp = send(&router, "POST", "/api/staff/admin/groups", &admin, &csrf,
        json!({ "name": "ab", "enabled": true, "flags": {}, "deptIds": [] })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json_body(resp).await["error"]["fields"]["name"], "Group name must be at least 3 chars.");

    let resp = send(&router, "POST", "/api/staff/admin/groups", &admin, &csrf,
        json!({ "name": "M1 Agents", "enabled": true, "flags": {}, "deptIds": [] })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json_body(resp).await["error"]["fields"]["name"], "Group name already exists");
}

// --- AC-3: all eleven flags persist ------------------------------------------

#[tokio::test]
async fn eleven_flags_persist_including_net_new() {
    let pool = skip_if_no_db!("eleven_flags_persist_including_net_new");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    let resp = send(&router, "POST", "/api/staff/admin/groups", &admin, &csrf,
        json!({ "name": format!("Full {}", nonce()), "enabled": true, "flags": all_flags_on(), "deptIds": [] })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let id = json_body(resp).await["id"].as_i64().unwrap();

    let detail = json_body(get(&router, &format!("/api/staff/admin/groups/{id}"), &admin).await).await;
    for f in ["can_manage_faq", "can_manage_premade", "can_ban_emails", "can_view_staff_stats",
              "can_create_tickets", "can_close_tickets", "can_delete_tickets"] {
        assert_eq!(detail["flags"][f], true, "flag {f} persisted");
    }
}

// AC-3 (bootstrap): the four net-new flags flow to /api/staff/me for an admin
// whose group carries them (the seeded Administrators group).
#[tokio::test]
async fn net_new_flags_exposed_on_session_bootstrap() {
    let pool = skip_if_no_db!("net_new_flags_exposed_on_session_bootstrap");
    let router = dev_app(pool);
    let (admin, _) = login(&router, "admin", "Admin123!").await;
    let me = json_body(get(&router, "/api/staff/me", &admin).await).await;
    for f in ["can_manage_faq", "can_manage_premade", "can_ban_emails", "can_view_staff_stats"] {
        assert_eq!(me[f], true, "{f} exposed on bootstrap");
    }
}

// --- AC-4: dept-access full-replace ------------------------------------------

#[tokio::test]
async fn dept_access_full_replace() {
    let pool = skip_if_no_db!("dept_access_full_replace");
    let router = dev_app(pool.clone());
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    let d1: i32 = sqlx::query_scalar("SELECT dept_id FROM department WHERE dept_name='Support'").fetch_one(&pool).await.unwrap();
    let d2: i32 = sqlx::query_scalar("SELECT dept_id FROM department WHERE dept_name='Sales'").fetch_one(&pool).await.unwrap();

    // Create with dept [d1].
    let resp = send(&router, "POST", "/api/staff/admin/groups", &admin, &csrf,
        json!({ "name": format!("DA {}", nonce()), "enabled": true, "flags": {}, "deptIds": [d1] })).await;
    let id = json_body(resp).await["id"].as_i64().unwrap();
    let detail = json_body(get(&router, &format!("/api/staff/admin/groups/{id}"), &admin).await).await;
    assert_eq!(detail["dept_ids"], json!([d1]));

    // Replace with [d2].
    let resp = send(&router, "PUT", &format!("/api/staff/admin/groups/{id}"), &admin, &csrf,
        json!({ "name": detail["name"], "enabled": true, "flags": {}, "deptIds": [d2] })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let detail = json_body(get(&router, &format!("/api/staff/admin/groups/{id}"), &admin).await).await;
    assert_eq!(detail["dept_ids"], json!([d2]), "previously-checked removed, new inserted");
}

// --- AC-5: delete requires zero members --------------------------------------

#[tokio::test]
async fn delete_only_empty_groups() {
    let pool = skip_if_no_db!("delete_only_empty_groups");
    let router = dev_app(pool.clone());
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    // Empty group deletes cleanly + drops dept-access rows.
    let d1: i32 = sqlx::query_scalar("SELECT dept_id FROM department WHERE dept_name='Support'").fetch_one(&pool).await.unwrap();
    let resp = send(&router, "POST", "/api/staff/admin/groups", &admin, &csrf,
        json!({ "name": format!("Empty {}", nonce()), "enabled": true, "flags": {}, "deptIds": [d1] })).await;
    let empty_id = json_body(resp).await["id"].as_i64().unwrap();
    let resp = send(&router, "POST", "/api/staff/admin/groups/mass", &admin, &csrf,
        json!({ "action": "delete", "ids": [empty_id] })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["affected"], 1);
    let (g,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM groups WHERE group_id=$1").bind(empty_id as i32).fetch_one(&pool).await.unwrap();
    assert_eq!(g, 0);
    let (da,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM group_dept_access WHERE group_id=$1").bind(empty_id as i32).fetch_one(&pool).await.unwrap();
    assert_eq!(da, 0, "dept-access rows cascaded away");

    // The M1 Agents group has members → delete refused (group remains).
    let agents: i32 = sqlx::query_scalar("SELECT group_id FROM groups WHERE group_name='M1 Agents'").fetch_one(&pool).await.unwrap();
    let resp = send(&router, "POST", "/api/staff/admin/groups/mass", &admin, &csrf,
        json!({ "action": "delete", "ids": [agents] })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let j = json_body(resp).await;
    assert_eq!(j["affected"], 0);
    assert_eq!(j["message"], "Unable to delete selected groups");
    let (g,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM groups WHERE group_id=$1").bind(agents).fetch_one(&pool).await.unwrap();
    assert_eq!(g, 1, "group with members remains");
}

// --- AC-6: cannot mass-act on own group; empty selection → 422 ---------------

#[tokio::test]
async fn own_group_guard_and_empty_selection() {
    let pool = skip_if_no_db!("own_group_guard_and_empty_selection");
    let router = dev_app(pool.clone());
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    let own: i32 = sqlx::query_scalar(
        "SELECT s.group_id FROM staff s WHERE s.username='admin'").fetch_one(&pool).await.unwrap();

    // Empty → 422.
    let resp = send(&router, "POST", "/api/staff/admin/groups/mass", &admin, &csrf,
        json!({ "action": "disable", "ids": [] })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    for action in ["disable", "delete"] {
        let resp = send(&router, "POST", "/api/staff/admin/groups/mass", &admin, &csrf,
            json!({ "action": action, "ids": [own] })).await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY, "{action} own group");
        assert_eq!(
            json_body(resp).await["error"]["message"],
            "As an admin, you can't disable/delete a group you belong to - you might lockout all admins!"
        );
    }
}

// zero-member groups appear in the list (left-join) ---------------------------
#[tokio::test]
async fn zero_member_group_appears_in_list() {
    let pool = skip_if_no_db!("zero_member_group_appears_in_list");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let name = format!("Lonely {}", nonce());
    let resp = send(&router, "POST", "/api/staff/admin/groups", &admin, &csrf,
        json!({ "name": name, "enabled": true, "flags": {}, "deptIds": [] })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let list = json_body(get(&router, "/api/staff/admin/groups", &admin).await).await;
    let found = list.as_array().unwrap().iter().find(|g| g["name"] == name).expect("zero-member group listed");
    assert_eq!(found["member_count"], 0);
}
