//! HTTP-level integration tests for the collaborative ticket edit-lock
//! (US-M3-I2 / FS-021.18, FS-021.20, BS-021.3), driven through the router via
//! `oneshot` with the seeded two-agent fixture (`agent` + `agent2`). Skipped
//! (pass, log line) when `TEST_DATABASE_URL` is unset.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test staff_lock
//! ```
//!
//! @implements FS-021.18: acquire/renew idempotent + lock exposed on detail load.
//! @implements FS-021.20: "locked by another staff" 409 + named banner on detail.
//! @implements BS-021.3: reply blocked while another agent holds a live lock.

use api::state::MailerHandle;
use api::{app, AppEnv, AppState};
use axum::body::Body;
use axum::Router;
use http::header::{COOKIE, SET_COOKIE};
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use ost_core::ticket::{create_ticket, NewTicket, NewTicketInput};
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
    app(
        AppState::with_pool(pool)
            .with_app_env(AppEnv::Development)
            .with_mailer(MailerHandle::stub()),
        ORIGIN,
    )
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
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

async fn login(router: &Router, username: &str, password: &str) -> (String, String) {
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/staff/login")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"username":"{username}","password":"{password}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    (
        set_cookie_value(&resp, "ost_staff_sess").unwrap(),
        set_cookie_value(&resp, "XSRF-TOKEN-STAFF").unwrap(),
    )
}

async fn make_ticket(pool: &PgPool) -> i64 {
    let nt = NewTicket::validated(NewTicketInput {
        email: "locktest@example.com".into(),
        name: "Lock Tester".into(),
        subject: "needs lock".into(),
        body: "first message".into(),
        source: Some("Web".into()),
        dept_id: None,
    })
    .unwrap();
    create_ticket(pool, &nt).await.unwrap().ticket_id
}

/// Remove the ticket (+ its lock/thread) so tests don't pollute the shared DB.
async fn cleanup(pool: &PgPool, id: i64) {
    for stmt in [
        "DELETE FROM ticket_lock WHERE ticket_id = $1",
        "DELETE FROM ticket_thread WHERE ticket_id = $1",
        "DELETE FROM ticket WHERE ticket_id = $1",
    ] {
        sqlx::query(stmt).bind(id).execute(pool).await.unwrap();
    }
}

fn cookie_header(sess: &str, csrf: &str) -> String {
    format!("ost_staff_sess={sess}; XSRF-TOKEN-STAFF={csrf}")
}

async fn post_lock(router: &Router, id: i64, sess: &str, csrf: &str) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/staff/tickets/{id}/lock"))
                .header(COOKIE, cookie_header(sess, csrf))
                .header("X-CSRFToken", csrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn delete_lock(router: &Router, id: i64, sess: &str, csrf: &str) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/staff/tickets/{id}/lock"))
                .header(COOKIE, cookie_header(sess, csrf))
                .header("X-CSRFToken", csrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn get_detail(router: &Router, id: i64, sess: &str) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/staff/tickets/{id}"))
                .header(COOKIE, format!("ost_staff_sess={sess}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn post_reply(router: &Router, id: i64, sess: &str, csrf: &str) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/staff/tickets/{id}/reply"))
                .header("content-type", "application/json")
                .header(COOKIE, cookie_header(sess, csrf))
                .header("X-CSRFToken", csrf)
                .body(Body::from(r#"{"body":"on it"}"#))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Same agent auto-acquiring the lock twice (the double-invoke scenario) both
/// succeed — no spurious lock failure (DEFECT 1).
///
/// @implements FS-021.18: idempotent acquire over HTTP.
#[tokio::test]
async fn same_agent_lock_twice_succeeds() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping same_agent_lock_twice_succeeds");
        return;
    };
    let router = dev_app(pool.clone());
    let id = make_ticket(&pool).await;
    let (sess, csrf) = login(&router, "agent", "Agent123!").await;

    let r1 = post_lock(&router, id, &sess, &csrf).await;
    assert_eq!(r1.status(), StatusCode::OK, "first acquire ok");
    let r2 = post_lock(&router, id, &sess, &csrf).await;
    assert_eq!(r2.status(), StatusCode::OK, "second acquire (renew) ok — not a spurious failure");

    cleanup(&pool, id).await;
}

/// The two-agent conflict: agent2 holds the lock, so agent's reply is rejected
/// (409) and agent's detail load reports `lock.locked_by_other` + agent2's name.
///
/// @implements FS-021.20: named "locked by another staff" signal.
/// @implements BS-021.3: reply blocked while another agent holds the lock.
#[tokio::test]
async fn other_agent_lock_blocks_reply_and_names_holder() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping other_agent_lock_blocks_reply_and_names_holder");
        return;
    };
    let router = dev_app(pool.clone());
    let id = make_ticket(&pool).await;

    let (sess2, csrf2) = login(&router, "agent2", "Agent234!").await;
    let (sess1, csrf1) = login(&router, "agent", "Agent123!").await;

    // agent2 grabs the lock.
    assert_eq!(post_lock(&router, id, &sess2, &csrf2).await.status(), StatusCode::OK);

    // agent's detail load surfaces the lock contract with agent2's name.
    let detail = json_body(get_detail(&router, id, &sess1).await).await;
    assert_eq!(detail["lock"]["locked_by_other"], serde_json::Value::Bool(true));
    assert_eq!(detail["lock"]["locked_by_name"], "Agent Two");
    assert!(detail["lock"]["expires_at"].is_string(), "expires_at present");

    // agent's reply is denied (409 conflict) while agent2 holds the lock.
    let denied = post_reply(&router, id, &sess1, &csrf1).await;
    assert_eq!(denied.status(), StatusCode::CONFLICT);
    let body = json_body(denied).await;
    assert!(
        body["error"]["message"].as_str().unwrap_or("").contains("locked"),
        "409 envelope mentions the lock: {body:?}"
    );

    // agent2 (the holder) sees its OWN lock (not "other") and CAN reply.
    let d2 = json_body(get_detail(&router, id, &sess2).await).await;
    assert_eq!(d2["lock"]["locked_by_other"], serde_json::Value::Bool(false));
    assert_eq!(post_reply(&router, id, &sess2, &csrf2).await.status(), StatusCode::OK);

    cleanup(&pool, id).await;
}

/// After the holder releases the lock, the other agent can reply again.
///
/// @implements FS-021.18: release frees the ticket for another agent.
#[tokio::test]
async fn reply_allowed_after_holder_releases() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping reply_allowed_after_holder_releases");
        return;
    };
    let router = dev_app(pool.clone());
    let id = make_ticket(&pool).await;

    let (sess2, csrf2) = login(&router, "agent2", "Agent234!").await;
    let (sess1, csrf1) = login(&router, "agent", "Agent123!").await;

    assert_eq!(post_lock(&router, id, &sess2, &csrf2).await.status(), StatusCode::OK);
    assert_eq!(post_reply(&router, id, &sess1, &csrf1).await.status(), StatusCode::CONFLICT);

    // Holder releases → agent can now reply.
    assert_eq!(delete_lock(&router, id, &sess2, &csrf2).await.status(), StatusCode::OK);
    assert_eq!(post_reply(&router, id, &sess1, &csrf1).await.status(), StatusCode::OK);

    cleanup(&pool, id).await;
}
