//! Integration tests for the TS-M1-C1 staff auth routes (login/logout/me).
//! Driven through the router via `oneshot`; skipped (pass, log line) when
//! `TEST_DATABASE_URL` is unset.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test staff_auth
//! ```
//!
//! @implements BS-002: valid credentials → staff session + profile (AC-1);
//!   invalid credentials → 401, no session (AC-2); staff route without a session
//!   → 401 (AC-3).

use api::{app, AppEnv, AppState};
use axum::body::Body;
use axum::Router;
use http::header::{COOKIE, SET_COOKIE};
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::postgres::PgPool;
use tower::ServiceExt;

const ORIGIN: &str = "http://localhost:3702";

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

/// Return the full `Set-Cookie` header value whose cookie name matches `name`.
fn set_cookie_raw(resp: &http::Response<Body>, name: &str) -> Option<String> {
    for hv in resp.headers().get_all(SET_COOKIE) {
        let s = hv.to_str().ok()?;
        if s.split(';').next().unwrap_or("").starts_with(&format!("{name}=")) {
            return Some(s.to_string());
        }
    }
    None
}

fn set_cookie_value(resp: &http::Response<Body>, name: &str) -> Option<String> {
    let raw = set_cookie_raw(resp, name)?;
    let first = raw.split(';').next()?;
    first.split_once('=').map(|(_, v)| v.to_string())
}

async fn json_body(resp: http::Response<Body>) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

async fn staff_login(router: &Router, body: &str) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/staff/login")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

// --- AC-1: valid credentials → session + profile ----------------------------

#[tokio::test]
async fn valid_login_sets_cookies_and_me_returns_profile() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping valid_login_sets_cookies_and_me_returns_profile");
        return;
    };
    let router = dev_app(pool);

    let login = staff_login(&router, r#"{"username":"agent","password":"Agent123!"}"#).await;
    assert_eq!(login.status(), StatusCode::OK);

    // ost_staff_sess is HttpOnly; XSRF-TOKEN-STAFF is present (and not HttpOnly).
    let sess_raw = set_cookie_raw(&login, "ost_staff_sess").expect("session cookie set");
    assert!(sess_raw.contains("HttpOnly"), "session cookie must be HttpOnly");
    let csrf_raw = set_cookie_raw(&login, "XSRF-TOKEN-STAFF").expect("csrf cookie set");
    assert!(!csrf_raw.contains("HttpOnly"), "csrf cookie must be readable by JS");

    let sess = set_cookie_value(&login, "ost_staff_sess").unwrap();
    let me = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/staff/me")
                .header(COOKIE, format!("ost_staff_sess={sess}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(me.status(), StatusCode::OK);
    let json = json_body(me).await;
    assert!(json["id"].is_i64() || json["id"].is_u64(), "id present");
    assert_eq!(json["username"], "agent");
    assert!(json["name"].is_string(), "name present");
    assert!(json["deptId"].is_i64() || json["deptId"].is_u64(), "deptId present");
}

// --- AC-2: invalid credentials → 401, no session ----------------------------

#[tokio::test]
async fn invalid_login_is_401_and_sets_no_session() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping invalid_login_is_401_and_sets_no_session");
        return;
    };
    let router = dev_app(pool);

    let resp = staff_login(&router, r#"{"username":"agent","password":"wrong"}"#).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert!(set_cookie_raw(&resp, "ost_staff_sess").is_none(), "no session cookie on failure");
    let json = json_body(resp).await;
    assert!(json["error"]["message"].is_string(), "shared error envelope");
}

// --- AC-3: staff route without a session → 401 ------------------------------

#[tokio::test]
async fn me_without_session_is_401() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping me_without_session_is_401");
        return;
    };
    let router = dev_app(pool);
    let resp = router
        .oneshot(Request::builder().uri("/api/staff/me").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
