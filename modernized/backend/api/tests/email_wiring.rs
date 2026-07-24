//! Integration tests for TS-M2-E3 — autoresponse on create + notification on
//! reply, wired through the active mailer.
//!
//! DB-backed (skip-pass when `TEST_DATABASE_URL` is unset). The Mailpit-delivery
//! cases additionally skip-pass when `MAILPIT_URL` is unset (ROADMAP M2 §5). The
//! stub-path cases (AC-3 no-mail-on-failure, AC-4 dev-mailbox fallback) force the
//! stub mailer so they are deterministic regardless of an ambient `SMTP_HOST`.
//!
//! ```sh
//! docker compose up -d mailpit
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//! MAILPIT_URL=http://localhost:3705 \
//!   cargo test -p api --test email_wiring
//! ```
//!
//! @implements FS-011.12: autoresponse on create (always-send §12).
//! @implements FS-021.3: notification on reply (always-send §12).

use std::sync::Arc;

use api::state::MailerHandle;
use api::{app, AppEnv, AppState};
use axum::body::Body;
use axum::Router;
use http::header::{COOKIE, SET_COOKIE};
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use ost_core::{SmtpConfig, SmtpMailer, SmtpTls, StubMailer};
use sqlx::postgres::PgPool;
use tower::ServiceExt;

const ORIGIN: &str = "http://localhost:3702";
const SMTP_HOST: &str = "localhost";
const SMTP_PORT: u16 = 3704;
const SMTP_FROM: &str = "support@example.com";

fn test_db_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok().filter(|s| !s.trim().is_empty())
}
fn mailpit_url() -> Option<String> {
    std::env::var("MAILPIT_URL").ok().filter(|s| !s.trim().is_empty())
}

async fn seeded_pool() -> Option<PgPool> {
    let url = test_db_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    tools::seed(&pool).await.expect("seed");
    Some(pool)
}

/// App forcing the recording stub mailer (deterministic dev-mailbox assertions).
fn stub_app(pool: PgPool) -> Router {
    app(
        AppState::with_pool(pool)
            .with_app_env(AppEnv::Development)
            .with_mailer(MailerHandle::stub()),
        ORIGIN,
    )
}

/// App with the real SMTP mailer active (delivers to Mailpit).
fn smtp_app(pool: PgPool) -> Router {
    let smtp = SmtpMailer::new(SmtpConfig {
        host: SMTP_HOST.to_string(),
        port: SMTP_PORT,
        from: SMTP_FROM.to_string(),
        from_name: None,
        user: None,
        pass: None,
        tls: SmtpTls::None,
    })
    .expect("build SmtpMailer");
    let handle = MailerHandle::from_parts(Arc::new(smtp), Arc::new(StubMailer::new()));
    app(
        AppState::with_pool(pool)
            .with_app_env(AppEnv::Development)
            .with_mailer(handle),
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

async fn create_ticket_json(router: &Router, name: &str, email: &str) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tickets")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"name":"{name}","email":"{email}","subject":"Hi","message":"hello"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn login(router: &Router) -> (String, String) {
    let resp = router
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
    (
        set_cookie_value(&resp, "ost_staff_sess").unwrap(),
        set_cookie_value(&resp, "XSRF-TOKEN-STAFF").unwrap(),
    )
}

async fn ticket_id_for_number(pool: &PgPool, number: i64) -> i64 {
    sqlx::query_scalar(r#"SELECT ticket_id FROM ticket WHERE "ticketID" = $1"#)
        .bind(number)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn post_reply(router: &Router, id: i64, cookie: &str, csrf: &str) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/staff/tickets/{id}/reply"))
                .header("content-type", "application/json")
                .header(COOKIE, format!("ost_staff_sess={cookie}; XSRF-TOKEN-STAFF={csrf}"))
                .header("X-CSRFToken", csrf)
                .body(Body::from(r#"{"body":"on it"}"#))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn mailbox(router: &Router) -> serde_json::Value {
    let resp = router
        .clone()
        .oneshot(Request::builder().uri("/api/dev/mailbox").body(Body::empty()).unwrap())
        .await
        .unwrap();
    json_body(resp).await
}

fn unique_email(tag: &str) -> String {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{tag}-{nonce}@example.com")
}

// --- AC-1 + AC-2: SMTP-active create + reply deliver to Mailpit -------------

#[tokio::test]
async fn create_and_reply_deliver_to_mailpit() {
    let (Some(pool), Some(base)) = (seeded_pool().await, mailpit_url()) else {
        eprintln!("TEST_DATABASE_URL / MAILPIT_URL unset — skipping create_and_reply_deliver_to_mailpit");
        return;
    };
    let router = smtp_app(pool.clone());
    let to = unique_email("mia");

    // AC-1: open a ticket → autoresponse delivered to the requester.
    let resp = create_ticket_json(&router, "Mia", &to).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let number = json_body(resp).await["ticketNumber"].as_i64().unwrap();

    let auto = find_message_to(&base, &to).await.expect("autoresponse delivered");
    assert!(
        auto["Subject"].as_str().unwrap_or("").contains(&number.to_string()),
        "autoresponse subject references the new ticket number"
    );
    let id = ticket_id_for_number(&pool, number).await;

    // AC-2: an agent reply → notification delivered to the requester.
    let (cookie, csrf) = login(&router).await;
    let reply = post_reply(&router, id, &cookie, &csrf).await;
    assert_eq!(reply.status(), StatusCode::OK);

    // Two messages now addressed to the requester (autoresponse + notification).
    let count = count_messages_to(&base, &to).await;
    assert!(count >= 2, "both the autoresponse and the notification were delivered (got {count})");
}

// --- AC-3: a failing create sends no mail -----------------------------------

#[tokio::test]
async fn failing_create_sends_no_mail() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping failing_create_sends_no_mail");
        return;
    };
    let router = stub_app(pool);
    // Invalid: blank name/subject/message + bad email ⇒ 422.
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tickets")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"","email":"bad","subject":"","message":""}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // No intent recorded in the dev mailbox (send fires only after a commit).
    let arr = mailbox(&router).await;
    assert_eq!(arr.as_array().unwrap().len(), 0, "no mail recorded for a failed create");
}

// --- AC-4: stub fallback records both events in the dev mailbox -------------

#[tokio::test]
async fn stub_fallback_records_both_events() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping stub_fallback_records_both_events");
        return;
    };
    let router = stub_app(pool.clone());
    let to = unique_email("fallback");

    // Create → records the autoresponse intent.
    let resp = create_ticket_json(&router, "Fallback", &to).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let number = json_body(resp).await["ticketNumber"].as_i64().unwrap();
    let id = ticket_id_for_number(&pool, number).await;

    // Reply → records the notification intent.
    let (cookie, csrf) = login(&router).await;
    assert_eq!(post_reply(&router, id, &cookie, &csrf).await.status(), StatusCode::OK);

    let arr = mailbox(&router).await;
    let to_count = arr
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["to"].as_str() == Some(to.as_str()))
        .count();
    assert_eq!(to_count, 2, "both autoresponse + notification recorded for {to}");
}

// --- AC (loop suppression): daemon/postmaster requester gets no autoresponse -

#[tokio::test]
async fn daemon_requester_autoresponse_suppressed() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping daemon_requester_autoresponse_suppressed");
        return;
    };
    let router = stub_app(pool);
    let to = "mailer-daemon@example.com";
    let resp = create_ticket_json(&router, "Daemon", to).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let arr = mailbox(&router).await;
    assert!(
        !arr.as_array().unwrap().iter().any(|m| m["to"].as_str() == Some(to)),
        "no autoresponse recorded for a daemon requester (loop suppression)"
    );
}

// --- Mailpit helpers --------------------------------------------------------

async fn find_message_to(base: &str, to: &str) -> Option<serde_json::Value> {
    for _ in 0..20 {
        let list: serde_json::Value = reqwest::get(format!("{base}/api/v1/messages"))
            .await
            .ok()?
            .json()
            .await
            .ok()?;
        if let Some(found) = list["messages"].as_array().and_then(|arr| {
            arr.iter().find(|m| {
                m["To"]
                    .as_array()
                    .is_some_and(|tos| tos.iter().any(|t| t["Address"].as_str() == Some(to)))
            })
        }) {
            return Some(found.clone());
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    None
}

async fn count_messages_to(base: &str, to: &str) -> usize {
    // Allow the second (notification) message a moment to arrive.
    for _ in 0..20 {
        let list: serde_json::Value = reqwest::get(format!("{base}/api/v1/messages"))
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let n = list["messages"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter(|m| {
                        m["To"]
                            .as_array()
                            .is_some_and(|tos| tos.iter().any(|t| t["Address"].as_str() == Some(to)))
                    })
                    .count()
            })
            .unwrap_or(0);
        if n >= 2 {
            return n;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    0
}
