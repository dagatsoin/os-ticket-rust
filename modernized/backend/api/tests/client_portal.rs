//! Integration tests for the TS-M1-D1 client auth + thread route + the dev
//! seed-ticket endpoint. Driven through the router via `oneshot`; skipped (pass,
//! log line) when `TEST_DATABASE_URL` is unset.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test client_portal
//! ```
//!
//! @implements FS-010.3: ticket# + email login (case-insensitive, generic error).
//! @implements FS-010 (security): client thread returns M + R only, never N.

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
    raw.split(';').next()?.split_once('=').map(|(_, v)| v.to_string())
}

async fn json_body(resp: http::Response<Body>) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

/// Seed a ticket via the dev endpoint; returns (ticketNumber, email).
async fn seed_ticket(router: &Router, body: &str) -> (String, String) {
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/dev/seed-ticket")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED, "seed-ticket creates a ticket");
    let json = json_body(resp).await;
    (
        json["ticketNumber"].as_i64().unwrap().to_string(),
        json["email"].as_str().unwrap().to_string(),
    )
}

async fn client_login(router: &Router, number: &str, email: &str) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/client/login")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"ticketNumber":"{number}","email":"{email}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap()
}

// --- AC-1: correct ticket# + email → session; wrong email → rejected --------

#[tokio::test]
async fn correct_credentials_establish_session_wrong_email_rejected() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping correct_credentials_establish_session...");
        return;
    };
    let router = dev_app(pool);
    let (number, email) = seed_ticket(&router, "{}").await;

    // Wrong email → 401, no session cookie.
    let wrong = client_login(&router, &number, "wrong@example.com").await;
    assert_eq!(wrong.status(), StatusCode::UNAUTHORIZED);
    assert!(set_cookie_raw(&wrong, "ost_client_sess").is_none(), "no session on failure");

    // Correct → 200 + ost_client_sess + XSRF-TOKEN-CLIENT cookies.
    let ok = client_login(&router, &number, &email).await;
    assert_eq!(ok.status(), StatusCode::OK);
    assert!(set_cookie_raw(&ok, "ost_client_sess").is_some(), "session cookie set");
    assert!(set_cookie_raw(&ok, "XSRF-TOKEN-CLIENT").is_some(), "csrf cookie set");
}

// --- FS-010.3: case-insensitive email match ---------------------------------

#[tokio::test]
async fn email_match_is_case_insensitive() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping email_match_is_case_insensitive");
        return;
    };
    let router = dev_app(pool);
    let (number, email) = seed_ticket(&router, "{}").await;

    let upper = email.to_uppercase();
    let resp = client_login(&router, &number, &upper).await;
    assert_eq!(resp.status(), StatusCode::OK, "uppercase email must still match");
}

// --- AC-2: thread route returns the session's own ticket + entries -----------

#[tokio::test]
async fn thread_route_returns_own_ticket_entries() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping thread_route_returns_own_ticket_entries");
        return;
    };
    let router = dev_app(pool);
    let (number, email) = seed_ticket(&router, r#"{"withReply":true}"#).await;

    let login = client_login(&router, &number, &email).await;
    let sess = set_cookie_value(&login, "ost_client_sess").unwrap();

    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/client/ticket")
                .header(COOKIE, format!("ost_client_sess={sess}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let ticket = json_body(resp).await;
    assert_eq!(ticket["number"].as_i64().unwrap().to_string(), number, "own ticket only");
    let entries = ticket["entries"].as_array().unwrap();
    // M (original) + R (seeded reply), in order.
    let types: Vec<&str> = entries.iter().map(|e| e["threadType"].as_str().unwrap()).collect();
    assert_eq!(types, vec!["M", "R"]);
}

// --- AC-3: client thread route has no path param + staff route denied --------

#[tokio::test]
async fn client_cannot_reach_staff_routes() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping client_cannot_reach_staff_routes");
        return;
    };
    let router = dev_app(pool);
    let (number, email) = seed_ticket(&router, "{}").await;
    let login = client_login(&router, &number, &email).await;
    let sess = set_cookie_value(&login, "ost_client_sess").unwrap();

    // The client cookie on a staff route is rejected (realm isolation).
    let staff = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/staff/tickets")
                .header(COOKIE, format!("ost_client_sess={sess}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(staff.status(), StatusCode::UNAUTHORIZED, "client cannot reach staff routes");
}

// --- AC-4 (security): client thread returns M + R, NEVER N -------------------

#[tokio::test]
async fn client_thread_never_returns_internal_note() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping client_thread_never_returns_internal_note");
        return;
    };
    let router = dev_app(pool.clone());
    // Seed WITH a reply AND an internal note.
    let (number, email) = seed_ticket(&router, r#"{"withReply":true,"withNote":true}"#).await;

    // Sanity: the ticket really does have an N note persisted.
    let note_count: i64 = sqlx::query_scalar(
        r#"SELECT count(*) FROM ticket_thread tt
           JOIN ticket t ON t.ticket_id = tt.ticket_id
           WHERE t."ticketID" = $1 AND tt.thread_type = 'N'"#,
    )
    .bind(number.parse::<i64>().unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(note_count, 1, "the seeded ticket has an N note");

    let login = client_login(&router, &number, &email).await;
    let sess = set_cookie_value(&login, "ost_client_sess").unwrap();
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/client/ticket")
                .header(COOKIE, format!("ost_client_sess={sess}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let ticket = json_body(resp).await;
    let types: Vec<&str> = ticket["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["threadType"].as_str().unwrap())
        .collect();
    assert!(!types.contains(&"N"), "internal note must NEVER reach the client: {types:?}");
    assert_eq!(types, vec!["M", "R"], "only M + R returned");
}

// --- dev seed-ticket is disabled in production ------------------------------

#[tokio::test]
async fn seed_ticket_is_disabled_in_production() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping seed_ticket_is_disabled_in_production");
        return;
    };
    let prod = app(AppState::with_pool(pool).with_app_env(AppEnv::Production), ORIGIN);
    let resp = prod
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/dev/seed-ticket")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND, "dev seed-ticket invisible in production");
}
