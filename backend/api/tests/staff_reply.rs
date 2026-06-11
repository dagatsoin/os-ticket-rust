//! Integration tests for the TS-M1-C3 staff reply route. Driven through the
//! router via `oneshot`; skipped (pass, log line) when `TEST_DATABASE_URL` is
//! unset.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test staff_reply
//! ```
//!
//! @implements BS-021: append R, pure append (no status mutation), authored by
//!   the agent (AC-1/AC-2).
//! @implements FS-040: notification recorded post-commit, via dev mailbox (AC-3).

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
    // Force the stub mailer so the dev-mailbox assertions are deterministic
    // regardless of an ambient SMTP_HOST in the test shell (TS-M2-E3).
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

/// Log in as a username/password, returning (session-cookie, csrf-token).
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

async fn make_ticket(pool: &PgPool, email: &str) -> i64 {
    let nt = NewTicket::validated(NewTicketInput {
        email: email.into(),
        name: "Reply Tester".into(),
        subject: "needs reply".into(),
        body: "first message".into(),
        source: Some("Web".into()),
        dept_id: None,
    })
    .unwrap();
    create_ticket(pool, &nt).await.unwrap().ticket_id
}

async fn post_reply(
    router: &Router,
    id: i64,
    cookie: &str,
    csrf: Option<&str>,
    body: &str,
) -> http::Response<Body> {
    // The double-submit CSRF check needs BOTH the XSRF cookie and the matching
    // header, so send the CSRF cookie alongside the session cookie.
    let cookie_header = match csrf {
        Some(c) => format!("ost_staff_sess={cookie}; XSRF-TOKEN-STAFF={c}"),
        None => format!("ost_staff_sess={cookie}"),
    };
    let mut b = Request::builder()
        .method("POST")
        .uri(format!("/api/staff/tickets/{id}/reply"))
        .header("content-type", "application/json")
        .header(COOKIE, cookie_header);
    if let Some(c) = csrf {
        b = b.header("X-CSRFToken", c);
    }
    router.clone().oneshot(b.body(Body::from(body.to_string())).unwrap()).await.unwrap()
}

// --- AC-1/AC-2: reply appends R, returns updated thread, status unchanged ----

#[tokio::test]
async fn reply_appends_r_returns_thread_and_keeps_status_open() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping reply_appends_r...");
        return;
    };
    let router = dev_app(pool.clone());
    let id = make_ticket(&pool, "ac1@example.com").await;

    let (cookie, csrf) = login(&router, "agent", "Agent123!").await;
    let resp = post_reply(
        &router,
        id,
        &cookie,
        Some(&csrf),
        r#"{"body":"We are looking into it."}"#,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let detail = json_body(resp).await;

    let entries = detail["entries"].as_array().unwrap();
    let types: Vec<&str> = entries.iter().map(|e| e["threadType"].as_str().unwrap()).collect();
    assert_eq!(types, vec!["M", "R"], "new R appended after the original M");
    // AC-2: the R entry is authored by the seeded agent.
    let r = entries.iter().find(|e| e["threadType"] == "R").unwrap();
    assert_eq!(r["poster"], "Agent One", "R authored by the agent");

    // Pure append: status is unchanged (still open).
    let status: String = sqlx::query_scalar("SELECT status FROM ticket WHERE ticket_id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "open", "no status mutation in M1");
}

// --- AC-3: notification recorded post-commit, via dev mailbox ---------------

#[tokio::test]
async fn reply_records_notification_after_commit() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping reply_records_notification_after_commit");
        return;
    };
    // A single app instance so the mailbox sees the same mailer store. Force the
    // stub mailer so the dev mailbox records intents (deterministic vs SMTP_HOST).
    let state = AppState::with_pool(pool.clone())
        .with_app_env(AppEnv::Development)
        .with_mailer(MailerHandle::stub());
    let router = app(state, ORIGIN);
    let id = make_ticket(&pool, "mailbox@example.com").await;

    let (cookie, csrf) = login(&router, "agent", "Agent123!").await;
    let resp = post_reply(&router, id, &cookie, Some(&csrf), r#"{"body":"on it"}"#).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let mailbox = router
        .clone()
        .oneshot(Request::builder().uri("/api/dev/mailbox").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let recorded = json_body(mailbox).await;
    let arr = recorded.as_array().unwrap();
    assert!(
        arr.iter().any(|m| m["to"] == "mailbox@example.com"),
        "an intended notification to the requester was recorded after commit"
    );
}

// --- AC-4: denied without session (401) / without can_post_reply (403) ------

#[tokio::test]
async fn reply_requires_session_and_permission() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping reply_requires_session_and_permission");
        return;
    };
    let router = dev_app(pool.clone());
    let id = make_ticket(&pool, "ac4@example.com").await;

    // No session → 401.
    let no_sess = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/staff/tickets/{id}/reply"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"body":"x"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(no_sess.status(), StatusCode::UNAUTHORIZED);

    // A staff account whose group LACKS can_post_reply → 403.
    let dept_id: i32 = sqlx::query_scalar("SELECT dept_id FROM department WHERE dept_name='Support'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let group_id: i32 = sqlx::query_scalar(
        "INSERT INTO groups (group_name, group_enabled, can_create_tickets, can_post_reply)
         VALUES ('No Reply Group', true, true, false) RETURNING group_id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let passwd = ost_core::hash_password("Agent123!").unwrap();
    sqlx::query(
        "INSERT INTO staff (group_id, dept_id, username, firstname, lastname, passwd, isactive)
         VALUES ($1, $2, 'noreply_agent', 'No', 'Reply', $3, true)
         ON CONFLICT (username) DO UPDATE SET group_id = EXCLUDED.group_id, passwd = EXCLUDED.passwd",
    )
    .bind(group_id)
    .bind(dept_id)
    .bind(&passwd)
    .execute(&pool)
    .await
    .unwrap();

    let (cookie, csrf) = login(&router, "noreply_agent", "Agent123!").await;
    let denied = post_reply(&router, id, &cookie, Some(&csrf), r#"{"body":"nope"}"#).await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN, "lacks can_post_reply ⇒ 403");
}
