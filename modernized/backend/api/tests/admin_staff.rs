//! Integration tests for TS-M4-B1 — admin staff CRUD + protections.
//!
//! Driven through the router via `oneshot`; every test skips (pass + log line)
//! when `TEST_DATABASE_URL` is unset so DB-less CI stays green.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test admin_staff
//! ```
//!
//! @implements FS-031.1/.3/.4/.5: list/create/edit/mass + validation.
//! @implements BS-031-014: last-active-administrator protection (edit).
//! @implements BS-031-015/016: self-action + deletion side effects (mass).
//! @implements BS-030-14: add / remove team membership.

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
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

async fn login(router: &Router, user: &str, pass: &str) -> (String, String) {
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/staff/login")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "username": user, "password": pass }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "login {user} should succeed");
    let sess = set_cookie_value(&resp, "ost_staff_sess").expect("session cookie");
    let csrf = set_cookie_value(&resp, "XSRF-TOKEN-STAFF").expect("csrf cookie");
    (sess, csrf)
}

async fn get(router: &Router, uri: &str, sess: &str) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .header(COOKIE, format!("ost_staff_sess={sess}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn send(
    router: &Router,
    method: &str,
    uri: &str,
    sess: &str,
    csrf: &str,
    body: Value,
) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header(COOKIE, format!("ost_staff_sess={sess}; XSRF-TOKEN-STAFF={csrf}"))
                .header(CSRF_HEADER, csrf)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn dev(router: &Router, uri: &str, body: Value) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn me_id(router: &Router, sess: &str) -> i64 {
    let j = json_body(get(router, "/api/staff/me", sess).await).await;
    j["id"].as_i64().expect("me id")
}

async fn support_dept(pool: &PgPool) -> i32 {
    sqlx::query_scalar("SELECT dept_id FROM department WHERE dept_name = 'Support'")
        .fetch_one(pool)
        .await
        .unwrap()
}
async fn agents_group(pool: &PgPool) -> i32 {
    sqlx::query_scalar("SELECT group_id FROM groups WHERE group_name = 'M1 Agents'")
        .fetch_one(pool)
        .await
        .unwrap()
}

macro_rules! skip_if_no_db {
    ($name:expr) => {{
        let Some(pool) = seeded_pool().await else {
            eprintln!("TEST_DATABASE_URL unset — skipping {}", $name);
            return;
        };
        pool
    }};
}

fn nonce() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

// --- AC-1: list filter/sort/paginate; agent → 403 ----------------------------

#[tokio::test]
async fn list_filters_sorts_and_gates_non_admin() {
    let pool = skip_if_no_db!("list_filters_sorts_and_gates_non_admin");
    let router = dev_app(pool);
    dev(&router, "/api/dev/reset-staff", json!({})).await;

    let (admin, _) = login(&router, "admin", "Admin123!").await;
    let resp = get(&router, "/api/staff/admin/staff?q=age&sort=name&page=1", &admin).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let j = json_body(resp).await;
    assert!(j["pagination"]["page"].as_i64().is_some());
    assert!(j["pagination"]["total"].as_i64().unwrap() >= 2, "agent + agent2 match 'age'");
    for row in j["staff"].as_array().unwrap() {
        let hay = format!("{}{}", row["name"], row["username"]).to_lowercase();
        assert!(hay.contains("age"), "q filter honored: {hay}");
    }

    // Non-admin agent → 403.
    let (agent, _) = login(&router, "agent", "Agent123!").await;
    let resp = get(&router, "/api/staff/admin/staff", &agent).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// --- AC-1b: pagination via a bulk group filter -------------------------------

#[tokio::test]
async fn list_paginates_bulk_group() {
    let pool = skip_if_no_db!("list_paginates_bulk_group");
    let router = dev_app(pool);
    let (admin, _) = login(&router, "admin", "Admin123!").await;

    let seeded = json_body(dev(&router, "/api/dev/seed-staff-bulk", json!({ "count": 30 })).await).await;
    let gid = seeded["groupId"].as_i64().unwrap();

    let p1 = json_body(get(&router, &format!("/api/staff/admin/staff?gid={gid}&page=1"), &admin).await).await;
    assert_eq!(p1["pagination"]["per_page"], 25);
    assert_eq!(p1["pagination"]["total"], 30);
    assert_eq!(p1["staff"].as_array().unwrap().len(), 25);

    let p2 = json_body(get(&router, &format!("/api/staff/admin/staff?gid={gid}&page=2"), &admin).await).await;
    assert_eq!(p2["staff"].as_array().unwrap().len(), 5);
}

// --- AC-2: create validation -------------------------------------------------

#[tokio::test]
async fn create_rejects_dup_username_short_and_mismatched_password() {
    let pool = skip_if_no_db!("create_rejects_dup_username_short_and_mismatched_password");
    let router = dev_app(pool.clone());
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let gid = agents_group(&pool).await;
    let did = support_dept(&pool).await;

    // Duplicate username "agent".
    let resp = send(&router, "POST", "/api/staff/admin/staff", &admin, &csrf, json!({
        "username": "agent", "firstname": "A", "lastname": "B",
        "email": format!("x{}@ex.com", nonce()), "password": "secret1", "passwd2": "secret1",
        "groupId": gid, "deptId": did,
    })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let j = json_body(resp).await;
    assert!(j["error"]["fields"]["username"].is_string());

    // Password < 6.
    let resp = send(&router, "POST", "/api/staff/admin/staff", &admin, &csrf, json!({
        "username": format!("u{}", nonce()), "firstname": "A", "lastname": "B",
        "email": format!("y{}@ex.com", nonce()), "password": "abc", "passwd2": "abc",
        "groupId": gid, "deptId": did,
    })).await;
    let j = json_body(resp).await;
    assert_eq!(j["error"]["fields"]["password"], "Must be at least 6 characters");

    // Mismatch.
    let resp = send(&router, "POST", "/api/staff/admin/staff", &admin, &csrf, json!({
        "username": format!("v{}", nonce()), "firstname": "A", "lastname": "B",
        "email": format!("z{}@ex.com", nonce()), "password": "secret1", "passwd2": "secret2",
        "groupId": gid, "deptId": did,
    })).await;
    let j = json_body(resp).await;
    assert_eq!(j["error"]["fields"]["passwd2"], "Password(s) do not match");
}

#[tokio::test]
async fn create_succeeds_and_forces_password_change() {
    let pool = skip_if_no_db!("create_succeeds_and_forces_password_change");
    let router = dev_app(pool.clone());
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let gid = agents_group(&pool).await;
    let did = support_dept(&pool).await;
    let uname = format!("new{}", nonce());

    let resp = send(&router, "POST", "/api/staff/admin/staff", &admin, &csrf, json!({
        "username": uname, "firstname": "New", "lastname": "Hire",
        "email": format!("{uname}@ex.com"), "password": "secret1", "passwd2": "secret1",
        "groupId": gid, "deptId": did,
    })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let id = json_body(resp).await["id"].as_i64().unwrap();

    let (cp,): (bool,) = sqlx::query_as("SELECT change_passwd FROM staff WHERE staff_id = $1")
        .bind(id as i32).fetch_one(&pool).await.unwrap();
    assert!(cp, "temp password forces change on next login");
}

// --- AC-3: last-active-administrator protection (edit) -----------------------

#[tokio::test]
async fn edit_cannot_remove_or_lock_last_admin() {
    let pool = skip_if_no_db!("edit_cannot_remove_or_lock_last_admin");
    let router = dev_app(pool);
    dev(&router, "/api/dev/reset-staff", json!({})).await;
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let id = me_id(&router, &admin).await;

    // Clear isadmin → 422 with the pinned message on the isadmin field.
    let resp = send(&router, "PUT", &format!("/api/staff/admin/staff/{id}"), &admin, &csrf,
        json!({ "isadmin": false })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let j = json_body(resp).await;
    assert_eq!(
        j["error"]["fields"]["isadmin"],
        "Cowardly refusing to remove or lock out the only active administrator"
    );

    // Lock → same rejection.
    let resp = send(&router, "PUT", &format!("/api/staff/admin/staff/{id}"), &admin, &csrf,
        json!({ "isactive": false })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let j = json_body(resp).await;
    assert!(j["error"]["fields"]["isadmin"].is_string());
}

// --- AC-4: self-action blocked; positive lock of another admin succeeds -------

#[tokio::test]
async fn mass_blocks_self_and_empty_and_allows_locking_other_admin() {
    let pool = skip_if_no_db!("mass_blocks_self_and_empty_and_allows_locking_other_admin");
    let router = dev_app(pool.clone());
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let my_id = me_id(&router, &admin).await;

    // Empty ids → 422.
    let resp = send(&router, "POST", "/api/staff/admin/staff/mass", &admin, &csrf,
        json!({ "action": "lock", "ids": [] })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Self in selection (lock) → 422.
    let resp = send(&router, "POST", "/api/staff/admin/staff/mass", &admin, &csrf,
        json!({ "action": "lock", "ids": [my_id] })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        json_body(resp).await["error"]["message"],
        "You can not disable/delete yourself - you could be the only admin!"
    );
    // Self (delete) → 422.
    let resp = send(&router, "POST", "/api/staff/admin/staff/mass", &admin, &csrf,
        json!({ "action": "delete", "ids": [my_id] })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Seed a SECOND admin; locking it succeeds (self remains as the active admin,
    // so the "≥1 active admin remains" guard allows it).
    let uname = format!("admin2_{}", nonce());
    let seeded = json_body(dev(&router, "/api/dev/seed-staff", json!({
        "username": uname, "password": "Admin234!", "isadmin": true
    })).await).await;
    let admin2 = seeded["staffId"].as_i64().unwrap();
    let resp = send(&router, "POST", "/api/staff/admin/staff/mass", &admin, &csrf,
        json!({ "action": "lock", "ids": [admin2] })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let (active,): (bool,) = sqlx::query_as("SELECT isactive FROM staff WHERE staff_id = $1")
        .bind(admin2 as i32).fetch_one(&pool).await.unwrap();
    assert!(!active, "second admin locked");
}

// --- WARN-2: last-active-admin guard holds under the in-transaction count -----

#[tokio::test]
async fn mass_lock_preserves_last_active_admin_in_txn() {
    let pool = skip_if_no_db!("mass_lock_preserves_last_active_admin_in_txn");
    let router = dev_app(pool.clone());
    dev(&router, "/api/dev/reset-staff", json!({})).await;
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let my_id = me_id(&router, &admin).await;

    // Two additional ACTIVE admins.
    let seed_admin = |uname: String| {
        let router = router.clone();
        async move {
            json_body(dev(&router, "/api/dev/seed-staff", json!({
                "username": uname, "password": "Admin234!", "isadmin": true
            })).await).await["staffId"].as_i64().unwrap()
        }
    };
    let admin2 = seed_admin(format!("adm2_{}", nonce())).await;
    let admin3 = seed_admin(format!("adm3_{}", nonce())).await;

    // Lock BOTH other admins in one mass action. The in-txn count excludes the
    // selected ids and still finds the acting admin active → allowed, and the
    // invariant "≥1 active admin remains" holds after commit.
    let resp = send(&router, "POST", "/api/staff/admin/staff/mass", &admin, &csrf,
        json!({ "action": "lock", "ids": [admin2, admin3] })).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let active_admins: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM staff WHERE isadmin = true AND isactive = true")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(active_admins, 1, "exactly the acting admin remains active");
    let (a2, a3): (bool, bool) = (
        sqlx::query_scalar("SELECT isactive FROM staff WHERE staff_id = $1").bind(admin2 as i32).fetch_one(&pool).await.unwrap(),
        sqlx::query_scalar("SELECT isactive FROM staff WHERE staff_id = $1").bind(admin3 as i32).fetch_one(&pool).await.unwrap(),
    );
    assert!(!a2 && !a3, "both other admins locked");

    // With the acting admin now the sole active admin, you cannot reach zero:
    // selecting self is refused (self-protection) — the guard prevents lockout.
    let resp = send(&router, "POST", "/api/staff/admin/staff/mass", &admin, &csrf,
        json!({ "action": "lock", "ids": [my_id] })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // And a mass DELETE of the (already-locked) other admins commits atomically
    // with the same in-txn guard, leaving the acting admin intact.
    let resp = send(&router, "POST", "/api/staff/admin/staff/mass", &admin, &csrf,
        json!({ "action": "delete", "ids": [admin2, admin3] })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let still_admin: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM staff WHERE staff_id = $1 AND isadmin = true AND isactive = true")
        .bind(my_id as i32).fetch_one(&pool).await.unwrap();
    assert_eq!(still_admin, 1, "acting admin survived the mass delete");
}

// --- AC-5: add / remove team membership --------------------------------------

#[tokio::test]
async fn add_and_remove_team_membership() {
    let pool = skip_if_no_db!("add_and_remove_team_membership");
    let router = dev_app(pool.clone());
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let gid = agents_group(&pool).await;
    let did = support_dept(&pool).await;

    // A team + a fresh staff member.
    let team_id: i32 = sqlx::query_scalar(
        "INSERT INTO team (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET updated = now() RETURNING team_id")
        .bind(format!("Team {}", nonce())).fetch_one(&pool).await.unwrap();
    let uname = format!("tm{}", nonce());
    let resp = send(&router, "POST", "/api/staff/admin/staff", &admin, &csrf, json!({
        "username": uname, "firstname": "T", "lastname": "M", "email": format!("{uname}@ex.com"),
        "password": "secret1", "passwd2": "secret1", "groupId": gid, "deptId": did,
    })).await;
    let sid = json_body(resp).await["id"].as_i64().unwrap();

    // Add.
    let resp = send(&router, "POST", &format!("/api/staff/admin/staff/{sid}/teams"), &admin, &csrf,
        json!({ "teamId": team_id })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let (cnt,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM team_member WHERE team_id=$1 AND staff_id=$2")
        .bind(team_id).bind(sid as i32).fetch_one(&pool).await.unwrap();
    assert_eq!(cnt, 1);

    // Remove.
    let resp = send(&router, "DELETE", &format!("/api/staff/admin/staff/{sid}/teams/{team_id}"), &admin, &csrf, json!({})).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let (cnt,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM team_member WHERE team_id=$1 AND staff_id=$2")
        .bind(team_id).bind(sid as i32).fetch_one(&pool).await.unwrap();
    assert_eq!(cnt, 0);
}

// --- AC-6: deletion side effects ---------------------------------------------

#[tokio::test]
async fn delete_clears_assignments_and_membership() {
    let pool = skip_if_no_db!("delete_clears_assignments_and_membership");
    let router = dev_app(pool.clone());
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let gid = agents_group(&pool).await;
    let did = support_dept(&pool).await;

    // Create staff.
    let uname = format!("del{}", nonce());
    let resp = send(&router, "POST", "/api/staff/admin/staff", &admin, &csrf, json!({
        "username": uname, "firstname": "D", "lastname": "L", "email": format!("{uname}@ex.com"),
        "password": "secret1", "passwd2": "secret1", "groupId": gid, "deptId": did,
    })).await;
    let sid = json_body(resp).await["id"].as_i64().unwrap() as i32;

    // Open ticket assigned to them.
    let seeded = json_body(dev(&router, "/api/dev/seed-tickets", json!({
        "tickets": [ { "status": "open", "staffId": sid } ]
    })).await).await;
    let ticket_id = seeded["ticket_ids"][0].as_i64().unwrap();

    // Team membership.
    let team_id: i32 = sqlx::query_scalar(
        "INSERT INTO team (name) VALUES ($1) RETURNING team_id")
        .bind(format!("DT {}", nonce())).fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO team_member (team_id, staff_id) VALUES ($1,$2)")
        .bind(team_id).bind(sid).execute(&pool).await.unwrap();

    // Mass delete.
    let resp = send(&router, "POST", "/api/staff/admin/staff/mass", &admin, &csrf,
        json!({ "action": "delete", "ids": [sid] })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["affected"], 1);

    // Ticket unassigned.
    let assignee: Option<i32> = sqlx::query_scalar("SELECT staff_id FROM ticket WHERE ticket_id=$1")
        .bind(ticket_id).fetch_one(&pool).await.unwrap();
    assert!(assignee.is_none(), "open ticket unassigned");
    // Team membership gone.
    let (tm,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM team_member WHERE staff_id=$1")
        .bind(sid).fetch_one(&pool).await.unwrap();
    assert_eq!(tm, 0);
    // Staff row gone.
    let (s,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM staff WHERE staff_id=$1")
        .bind(sid).fetch_one(&pool).await.unwrap();
    assert_eq!(s, 0);
}
