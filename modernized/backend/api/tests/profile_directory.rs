//! Integration tests for TS-M4-B5 — own profile + password change + directory
//! + login-time password aging.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test profile_directory
//! ```
//!
//! @implements FS-031.9/.10/.12/.13 + BS-031-013/030/031/032.

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
async fn login_resp(router: &Router, user: &str, pass: &str) -> http::Response<Body> {
    router.clone().oneshot(Request::builder().method("POST").uri("/api/staff/login")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "username": user, "password": pass }).to_string())).unwrap())
        .await.unwrap()
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
        .header("content-type", "application/json").body(Body::from(body.to_string())).unwrap()).await.unwrap()
}
macro_rules! skip_if_no_db { ($n:expr) => {{ let Some(p)=seeded_pool().await else { eprintln!("skip {}", $n); return; }; p }}; }
fn nonce() -> u128 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() }

/// Insert a directory staff row directly (email/phone/mobile that seed-staff omits).
async fn insert_dir_staff(pool: &PgPool, first: &str, last: &str, email: &str, mobile: &str, visible: bool) -> i32 {
    let gid: i32 = sqlx::query_scalar("SELECT group_id FROM groups WHERE group_name='M1 Agents'").fetch_one(pool).await.unwrap();
    let did: i32 = sqlx::query_scalar("SELECT dept_id FROM department WHERE dept_name='Support'").fetch_one(pool).await.unwrap();
    sqlx::query_scalar(
        "INSERT INTO staff (group_id, dept_id, username, firstname, lastname, email, mobile, passwd, isactive, isvisible)
         VALUES ($1,$2,$3,$4,$5,$6,$7,'x',true,$8) RETURNING staff_id")
        .bind(gid).bind(did)
        .bind(format!("dir{}", nonce()))
        .bind(first).bind(last).bind(email).bind(mobile).bind(visible)
        .fetch_one(pool).await.unwrap()
}

// --- AC-1: profile loads authenticated staff; username read-only; tamper deny -

#[tokio::test]
async fn profile_self_only_username_readonly_and_tamper_guard() {
    let pool = skip_if_no_db!("profile_self_only_username_readonly_and_tamper_guard");
    let router = dev_app(pool);
    let (agent, csrf) = login(&router, "agent", "Agent123!").await;

    let me = json_body(get(&router, "/api/staff/profile", &agent).await).await;
    assert_eq!(me["username"], "agent");
    let self_id = me["id"].as_i64().unwrap();

    // Foreign id → Action Denied.
    let resp = send(&router, "PUT", "/api/staff/profile", &agent, &csrf,
        json!({ "id": self_id + 987654, "firstname": "X" })).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert_eq!(json_body(resp).await["error"]["message"], "Action Denied");

    // Username change ignored (own id ok).
    let resp = send(&router, "PUT", "/api/staff/profile", &agent, &csrf,
        json!({ "id": self_id, "username": "hacked", "firstname": "Agent" })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let after = json_body(get(&router, "/api/staff/profile", &agent).await).await;
    assert_eq!(after["username"], "agent", "username unchanged");
}

// --- AC-2: timezone update reflected on subsequent read ----------------------

#[tokio::test]
async fn profile_timezone_update_reflected() {
    let pool = skip_if_no_db!("profile_timezone_update_reflected");
    let router = dev_app(pool.clone());
    // Use a fresh account so we don't fight other tests over the shared agent.
    let uname = format!("tz{}", nonce());
    dev(&router, "/api/dev/seed-staff", json!({ "username": uname, "password": "Secret123" })).await;
    let (sess, csrf) = login(&router, &uname, "Secret123").await;

    let tzid: i32 = sqlx::query_scalar("SELECT id FROM timezone ORDER BY id LIMIT 1").fetch_one(&pool).await.unwrap();
    let resp = send(&router, "PUT", "/api/staff/profile", &sess, &csrf, json!({ "timezoneId": tzid })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let after = json_body(get(&router, "/api/staff/profile", &sess).await).await;
    assert_eq!(after["timezone_id"], tzid);
}

// --- AC-3: password change validation order + success clears forced-change ----

#[tokio::test]
async fn password_change_order_and_success() {
    let pool = skip_if_no_db!("password_change_order_and_success");
    let router = dev_app(pool.clone());
    let uname = format!("pw{}", nonce());
    let seeded = json_body(dev(&router, "/api/dev/seed-staff", json!({ "username": uname, "password": "Secret123" })).await).await;
    let sid = seeded["staffId"].as_i64().unwrap();
    // Force change on so success can be shown to clear it.
    dev(&router, "/api/dev/age-password", json!({ "staffId": sid, "days": 120 })).await;
    let (sess, csrf) = login(&router, &uname, "Secret123").await;

    let pw = |b: Value| send(&router, "PUT", "/api/staff/profile/password", &sess, &csrf, b);

    // Blank new → New password required.
    let r = pw(json!({ "current": "Secret123", "new": "", "confirm": "" })).await;
    assert_eq!(r.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json_body(r).await["error"]["fields"]["new"], "New password required");
    // <6 → Must be at least 6 characters.
    let r = pw(json!({ "current": "Secret123", "new": "abc", "confirm": "abc" })).await;
    assert_eq!(json_body(r).await["error"]["fields"]["new"], "Must be at least 6 characters");
    // Mismatch → Password(s) do not match.
    let r = pw(json!({ "current": "Secret123", "new": "NewPass1", "confirm": "NewPass2" })).await;
    assert_eq!(json_body(r).await["error"]["fields"]["confirm"], "Password(s) do not match");
    // Wrong current → Invalid current password!
    let r = pw(json!({ "current": "WRONG", "new": "NewPass1", "confirm": "NewPass1" })).await;
    assert_eq!(json_body(r).await["error"]["fields"]["current"], "Invalid current password!");
    // new == current → must differ.
    let r = pw(json!({ "current": "Secret123", "new": "Secret123", "confirm": "Secret123" })).await;
    assert_eq!(json_body(r).await["error"]["fields"]["new"], "New password MUST be different from the current password!");

    // Success.
    let r = pw(json!({ "current": "Secret123", "new": "NewPass1", "confirm": "NewPass1" })).await;
    assert_eq!(r.status(), StatusCode::OK);
    let (cp,): (bool,) = sqlx::query_as("SELECT change_passwd FROM staff WHERE staff_id=$1").bind(sid as i32).fetch_one(&pool).await.unwrap();
    assert!(!cp, "forced-change cleared on success");
    let (has_reset,): (bool,) =
        sqlx::query_as("SELECT passwdreset IS NOT NULL FROM staff WHERE staff_id=$1").bind(sid as i32).fetch_one(&pool).await.unwrap();
    assert!(has_reset, "passwdreset recorded");
    // New password now logs in.
    assert_eq!(login_resp(&router, &uname, "NewPass1").await.status(), StatusCode::OK);
}

// --- AC-4: directory search matching (numeric / email / name) ----------------

#[tokio::test]
async fn directory_search_matching() {
    let pool = skip_if_no_db!("directory_search_matching");
    let router = dev_app(pool.clone());
    let token = format!("zeb{}", nonce() % 100000);
    let email = format!("{token}@dir.example");
    let mobile = format!("77{}", nonce() % 100000000);
    insert_dir_staff(&pool, "Zebediah", &token, &email, &mobile, true).await;

    let (agent, _) = login(&router, "agent", "Agent123!").await;

    // Numeric → matches mobile.
    let j = json_body(get(&router, &format!("/api/staff/directory?q={mobile}"), &agent).await).await;
    assert!(j["staff"].as_array().unwrap().iter().any(|s| s["mobile"] == mobile), "numeric matches mobile");

    // Email → exact.
    let j = json_body(get(&router, &format!("/api/staff/directory?q={email}"), &agent).await).await;
    let arr = j["staff"].as_array().unwrap();
    assert!(arr.iter().any(|s| s["email"] == email));
    assert!(arr.iter().all(|s| s["email"] == email), "email is an exact filter");

    // Name substring → matches lastname.
    let j = json_body(get(&router, &format!("/api/staff/directory?q={token}"), &agent).await).await;
    assert!(j["staff"].as_array().unwrap().iter().any(|s| s["email"] == email));
}

// --- AC-5: only directory-visible staff listed -------------------------------

#[tokio::test]
async fn directory_lists_only_visible() {
    let pool = skip_if_no_db!("directory_lists_only_visible");
    let router = dev_app(pool.clone());
    let token = format!("vis{}", nonce() % 100000);
    let visible_email = format!("v{token}@dir.example");
    let hidden_email = format!("h{token}@dir.example");
    insert_dir_staff(&pool, "Vee", &token, &visible_email, "", true).await;
    insert_dir_staff(&pool, "Aitch", &token, &hidden_email, "", false).await;

    let (agent, _) = login(&router, "agent", "Agent123!").await;
    let j = json_body(get(&router, &format!("/api/staff/directory?q={token}"), &agent).await).await;
    let emails: Vec<String> = j["staff"].as_array().unwrap().iter().map(|s| s["email"].as_str().unwrap().to_string()).collect();
    assert!(emails.contains(&visible_email), "visible staff present");
    assert!(!emails.contains(&hidden_email), "hidden staff absent");
}

// directory is reachable by ANY staff (not admin-gated) -----------------------
#[tokio::test]
async fn directory_open_to_any_staff() {
    let pool = skip_if_no_db!("directory_open_to_any_staff");
    let router = dev_app(pool);
    let (agent, _) = login(&router, "agent", "Agent123!").await;
    let resp = get(&router, "/api/staff/directory", &agent).await;
    assert_eq!(resp.status(), StatusCode::OK, "plain agent may browse the directory");
}

// --- AC-6: login-time aging sets forced-change (non-admin); admin exempt ------

#[tokio::test]
async fn login_aging_forces_change_for_nonadmin_only() {
    let pool = skip_if_no_db!("login_aging_forces_change_for_nonadmin_only");
    let router = dev_app(pool.clone());

    // A fresh non-admin whose passwdreset is backdated WITHOUT the forced flag,
    // so only the login-time computation can set it.
    let uname = format!("age{}", nonce());
    let seeded = json_body(dev(&router, "/api/dev/seed-staff", json!({ "username": uname, "password": "Secret123" })).await).await;
    let sid = seeded["staffId"].as_i64().unwrap();
    dev(&router, "/api/dev/age-password", json!({ "staffId": sid, "days": 120, "forceChange": false })).await;

    // A fresh admin, likewise backdated without the forced flag.
    let adname = format!("agadm{}", nonce());
    let adseed = json_body(dev(&router, "/api/dev/seed-staff", json!({ "username": adname, "password": "Secret123", "isadmin": true })).await).await;
    let adid = adseed["staffId"].as_i64().unwrap();
    dev(&router, "/api/dev/age-password", json!({ "staffId": adid, "days": 120, "forceChange": false })).await;

    // Enable the aging window (1 month).
    dev(&router, "/api/dev/seed-config", json!({ "passwd_reset_period": 1 })).await;

    // pre-condition: neither is forced yet.
    let (cp0,): (bool,) = sqlx::query_as("SELECT change_passwd FROM staff WHERE staff_id=$1").bind(sid as i32).fetch_one(&pool).await.unwrap();
    assert!(!cp0, "non-admin not yet forced");

    // Non-admin logs in → forced-change set by the login-time computation.
    assert_eq!(login_resp(&router, &uname, "Secret123").await.status(), StatusCode::OK);
    let (cp,): (bool,) = sqlx::query_as("SELECT change_passwd FROM staff WHERE staff_id=$1").bind(sid as i32).fetch_one(&pool).await.unwrap();
    assert!(cp, "over-age non-admin forced on login");

    // Admin logs in → NOT forced (exempt).
    assert_eq!(login_resp(&router, &adname, "Secret123").await.status(), StatusCode::OK);
    let (acp,): (bool,) = sqlx::query_as("SELECT change_passwd FROM staff WHERE staff_id=$1").bind(adid as i32).fetch_one(&pool).await.unwrap();
    assert!(!acp, "admin exempt from login aging");

    // Restore the window so parallel/later logins aren't affected.
    dev(&router, "/api/dev/seed-config", json!({ "passwd_reset_period": 0 })).await;
}
