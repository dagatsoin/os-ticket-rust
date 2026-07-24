//! Integration tests for the TS-M1-A4b auth HTTP layer — sessions, CSRF, realm
//! gates, and the dev mailbox. Driven through the router via `oneshot` (no live
//! socket). Skipped (pass, with a log line) when `TEST_DATABASE_URL` is unset so
//! offline / DB-less CI stays green:
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test auth
//! ```
//!
//! @implements BS-001: CSRF accept/reject; cross-realm denial (AC-1, AC-2).
//! @implements FS-002.14: session-id regeneration on login (AC-3).
//! @implements FS-040: dev mailbox dev/prod gating (AC-6).

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

/// Connect + migrate + seed (so `agent`/`Agent123!` exists), returning a pool.
async fn seeded_pool() -> Option<PgPool> {
    let url = test_db_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    tools::seed(&pool).await.expect("seed");
    Some(pool)
}

/// Insert a ticket so client login (ticket# + email) has a target. Returns the
/// external ticket number + email used.
async fn insert_ticket(pool: &PgPool) -> (i64, String) {
    let dept_id: i32 = sqlx::query_scalar("SELECT dept_id FROM department WHERE dept_name='Support'")
        .fetch_one(pool)
        .await
        .unwrap();
    let ticket_number: i64 = 100000
        + (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            % 900000) as i64;
    let email = format!("client{ticket_number}@example.com");
    sqlx::query(r#"INSERT INTO ticket ("ticketID", dept_id, email) VALUES ($1, $2, $3)"#)
        .bind(ticket_number)
        .bind(dept_id)
        .bind(&email)
        .execute(pool)
        .await
        .unwrap();
    (ticket_number, email)
}

fn dev_app(pool: PgPool) -> Router {
    app(AppState::with_pool(pool).with_app_env(AppEnv::Development), ORIGIN)
}

/// Extract a cookie value by name from a response's `Set-Cookie` headers.
fn set_cookie_value(resp: &http::Response<Body>, name: &str) -> Option<String> {
    for hv in resp.headers().get_all(SET_COOKIE) {
        let s = hv.to_str().ok()?;
        let first = s.split(';').next().unwrap_or("");
        if let Some((k, v)) = first.split_once('=') {
            if k == name {
                return Some(v.to_string());
            }
        }
    }
    None
}

async fn json_body(resp: http::Response<Body>) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

/// POST /api/staff/login with the seeded credentials; returns the response.
async fn staff_login(router: &Router) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/staff/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":"agent","password":"Agent123!"}"#))
                .unwrap(),
        )
        .await
        .unwrap()
}

// --- AC-3: session-id regeneration on login --------------------------------

#[tokio::test]
async fn login_regenerates_session_id_and_seeds_csrf_cookie() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping login_regenerates_session_id_and_seeds_csrf_cookie");
        return;
    };
    let router = dev_app(pool);

    let first = staff_login(&router).await;
    assert_eq!(first.status(), StatusCode::OK);
    let sess1 = set_cookie_value(&first, "ost_staff_sess").expect("session cookie set");
    let csrf1 = set_cookie_value(&first, "XSRF-TOKEN-STAFF").expect("csrf cookie set");
    assert!(!sess1.is_empty() && !csrf1.is_empty());

    // A second login mints a DIFFERENT session id (fixation defence).
    let second = staff_login(&router).await;
    let sess2 = set_cookie_value(&second, "ost_staff_sess").expect("session cookie set");
    assert_ne!(sess1, sess2, "session id must be regenerated on each login");
}

#[tokio::test]
async fn staff_login_rejects_bad_password() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping staff_login_rejects_bad_password");
        return;
    };
    let router = dev_app(pool);
    let resp = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/staff/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":"agent","password":"wrong"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// --- AC-1: CSRF on an authenticated mutating route -------------------------
//
// Logout is an authenticated mutating (POST) route, so it exercises the CSRF
// double-submit gate identically to a staff reply.

#[tokio::test]
async fn authenticated_post_without_csrf_is_rejected_and_with_token_is_accepted() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping authenticated_post_without_csrf_is_rejected_and_with_token_is_accepted");
        return;
    };
    let router = dev_app(pool);

    let login = staff_login(&router).await;
    let sess = set_cookie_value(&login, "ost_staff_sess").unwrap();
    let csrf = set_cookie_value(&login, "XSRF-TOKEN-STAFF").unwrap();
    let cookie_header = format!("ost_staff_sess={sess}; XSRF-TOKEN-STAFF={csrf}");

    // Missing CSRF header → 403 (authenticated but CSRF fails).
    let missing = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/staff/logout")
                .header(COOKIE, &cookie_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::FORBIDDEN, "no CSRF header ⇒ 403");

    // Matching CSRF header → accepted (200).
    let ok = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/staff/logout")
                .header(COOKIE, &cookie_header)
                .header("X-CSRFToken", &csrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK, "matching CSRF token ⇒ accepted");
}

// --- AC-2: cross-realm denial ----------------------------------------------

#[tokio::test]
async fn client_session_is_denied_on_staff_route_and_vice_versa() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping client_session_is_denied_on_staff_route_and_vice_versa");
        return;
    };
    let (ticket_number, email) = insert_ticket(&pool).await;
    let router = dev_app(pool);

    // Establish a client session.
    let client_login = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/client/login")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"ticketNumber":"{ticket_number}","email":"{email}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(client_login.status(), StatusCode::OK);
    let client_sess = set_cookie_value(&client_login, "ost_client_sess").unwrap();
    let client_csrf = set_cookie_value(&client_login, "XSRF-TOKEN-CLIENT").unwrap();

    // Client cookie on a STAFF route (logout = staff realm gate) → 401.
    let cross = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/staff/logout")
                .header(COOKIE, format!("ost_client_sess={client_sess}"))
                .header("X-CSRFToken", &client_csrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        cross.status(),
        StatusCode::UNAUTHORIZED,
        "client session must not satisfy the staff realm gate"
    );

    // Staff cookie on a CLIENT route → 401.
    let staff_login_resp = staff_login(&router).await;
    let staff_sess = set_cookie_value(&staff_login_resp, "ost_staff_sess").unwrap();
    let staff_csrf = set_cookie_value(&staff_login_resp, "XSRF-TOKEN-STAFF").unwrap();
    let cross2 = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/client/logout")
                .header(COOKIE, format!("ost_staff_sess={staff_sess}"))
                .header("X-CSRFToken", &staff_csrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        cross2.status(),
        StatusCode::UNAUTHORIZED,
        "staff session must not satisfy the client realm gate"
    );
}

// --- AC-6: dev mailbox gating ----------------------------------------------

#[tokio::test]
async fn dev_mailbox_returns_json_in_dev_and_404_in_production() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping dev_mailbox_returns_json_in_dev_and_404_in_production");
        return;
    };

    // Dev: 200 + JSON array.
    let dev = app(
        AppState::with_pool(pool.clone()).with_app_env(AppEnv::Development),
        ORIGIN,
    );
    let resp = dev
        .oneshot(Request::builder().uri("/api/dev/mailbox").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    assert!(json.is_array(), "mailbox returns a JSON array in dev");

    // Production: 404 (endpoint invisible).
    let prod = app(
        AppState::with_pool(pool).with_app_env(AppEnv::Production),
        ORIGIN,
    );
    let resp = prod
        .oneshot(Request::builder().uri("/api/dev/mailbox").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND, "dev mailbox disabled in production");
}
