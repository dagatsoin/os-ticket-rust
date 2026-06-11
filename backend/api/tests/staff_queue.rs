//! Integration tests for the TS-M1-C2 staff queue + detail routes. Driven
//! through the router via `oneshot`; skipped (pass, log line) when
//! `TEST_DATABASE_URL` is unset.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test staff_queue
//! ```
//!
//! @implements BS-020: open queue, created DESC (AC-1).
//! @implements BS-021: detail thread created ASC incl M/R/N (AC-2).

use api::{app, AppEnv, AppState};
use axum::body::Body;
use axum::Router;
use http::header::{COOKIE, SET_COOKIE};
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use ost_core::ticket::{
    append_thread_entry, create_ticket, NewThreadEntry, NewTicket, NewTicketInput, ThreadType,
};
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
    serde_json::from_slice(&bytes).unwrap()
}

async fn staff_cookie(router: &Router) -> String {
    let login = router
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
        .unwrap();
    format!("ost_staff_sess={}", set_cookie_value(&login, "ost_staff_sess").unwrap())
}

/// Create an open ticket with the given subject; returns its internal id.
async fn make_ticket(pool: &PgPool, subject: &str) -> i64 {
    let nt = NewTicket::validated(NewTicketInput {
        email: "queue@example.com".into(),
        name: "Queue Tester".into(),
        subject: subject.into(),
        body: "first message".into(),
        source: Some("Web".into()),
        dept_id: None,
    })
    .unwrap();
    create_ticket(pool, &nt).await.unwrap().ticket_id
}

async fn get(router: &Router, uri: &str, cookie: Option<&str>) -> http::Response<Body> {
    let mut b = Request::builder().uri(uri);
    if let Some(c) = cookie {
        b = b.header(COOKIE, c);
    }
    router.clone().oneshot(b.body(Body::empty()).unwrap()).await.unwrap()
}

// --- AC-1: queue lists open tickets created DESC ----------------------------

#[tokio::test]
async fn queue_lists_open_tickets_newest_first() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping queue_lists_open_tickets_newest_first");
        return;
    };
    let router = dev_app(pool.clone());

    let older = make_ticket(&pool, "OLDER queue ticket").await;
    // Nudge the older ticket's created timestamp back so ordering is deterministic.
    sqlx::query("UPDATE ticket SET created = created - interval '1 minute' WHERE ticket_id = $1")
        .bind(older)
        .execute(&pool)
        .await
        .unwrap();
    let newer = make_ticket(&pool, "NEWER queue ticket").await;

    let cookie = staff_cookie(&router).await;
    let resp = get(&router, "/api/staff/tickets?status=open", Some(&cookie)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let arr = json_body(resp).await;
    let list = arr.as_array().unwrap();

    // Each item has number + subject + email + created.
    for item in list {
        assert!(item["number"].is_i64() || item["number"].is_u64());
        assert!(item["subject"].is_string());
        assert!(item["email"].is_string());
        assert!(item["created"].is_string());
    }
    // The newer ticket appears before the older one (created DESC).
    let pos = |id: i64| list.iter().position(|i| i["id"].as_i64() == Some(id)).unwrap();
    assert!(pos(newer) < pos(older), "newer ticket must sort before older");
}

// --- AC-2: detail thread created ASC, including N internal note -------------

#[tokio::test]
async fn detail_returns_full_thread_ascending_including_note() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping detail_returns_full_thread...");
        return;
    };
    let router = dev_app(pool.clone());

    let staff_id: i32 = sqlx::query_scalar("SELECT staff_id FROM staff WHERE username = 'agent'")
        .fetch_one(&pool)
        .await
        .unwrap();

    let id = make_ticket(&pool, "detail ticket").await;
    // Append a staff response (R) then an internal note (N).
    append_thread_entry(
        &pool,
        id,
        &NewThreadEntry::response("Agent One", Some(staff_id), "looking into it"),
    )
    .await
    .unwrap();
    let note = NewThreadEntry {
        thread_type: ThreadType::Note,
        poster: "Agent One".into(),
        staff_id: Some(staff_id),
        body: "internal: escalate".into(),
    };
    append_thread_entry(&pool, id, &note).await.unwrap();

    let cookie = staff_cookie(&router).await;
    let resp = get(&router, &format!("/api/staff/tickets/{id}"), Some(&cookie)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let detail = json_body(resp).await;
    let entries = detail["entries"].as_array().unwrap();

    // Chronological: M (first) then R then N.
    let types: Vec<&str> = entries.iter().map(|e| e["threadType"].as_str().unwrap()).collect();
    assert_eq!(types, vec!["M", "R", "N"], "created ASC, all types incl N");
}

// --- AC-3: both routes denied without a staff session -----------------------

#[tokio::test]
async fn queue_and_detail_require_a_session() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping queue_and_detail_require_a_session");
        return;
    };
    let router = dev_app(pool);
    let q = get(&router, "/api/staff/tickets?status=open", None).await;
    assert_eq!(q.status(), StatusCode::UNAUTHORIZED);
    let d = get(&router, "/api/staff/tickets/1", None).await;
    assert_eq!(d.status(), StatusCode::UNAUTHORIZED);
}
