//! Integration tests for the TS-M2-D4 canned reply wiring + isanswered (§3).
//! Driven through the router via `oneshot`; skipped (pass, log line) when
//! `TEST_DATABASE_URL` is unset.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   BLOB_ROOT=/tmp/ost-canned-reply-test \
//!   cargo test -p api --test canned_reply
//! ```
//!
//! @implements FS-022.14: a canned reply posts the substituted body + carries
//!   the canned attachments (AC-1) alongside an own file (AC-3).
//! @implements ROADMAP §3: any staff reply marks the ticket answered (AC-2).
//! @implements BS-021: a plain reply still posts, M1-unchanged (AC-4).

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
    // Force the policy this suite depends on (the seed's config keys are
    // ON CONFLICT DO NOTHING, and a sibling test binary mutates them on the
    // shared DB), so the own-file AC-3 case is order-independent.
    for (key, value) in [
        ("allow_attachments", "true"),
        ("allowed_filetypes", ".pdf,.png,.jpg,.txt,.doc"),
        ("max_file_size", "1048576"),
    ] {
        sqlx::query(
            "INSERT INTO config (key, value) VALUES ($1, $2)
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(key)
        .bind(value)
        .execute(&pool)
        .await
        .unwrap();
    }
    Some(pool)
}

fn dev_app(pool: PgPool) -> Router {
    app(
        AppState::with_pool(pool).with_app_env(AppEnv::Development),
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

async fn login(router: &Router) -> (String, String) {
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/staff/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"username":"agent","password":"Agent123!"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    (
        set_cookie_value(&resp, "ost_staff_sess").unwrap(),
        set_cookie_value(&resp, "XSRF-TOKEN-STAFF").unwrap(),
    )
}

async fn make_ticket(pool: &PgPool, email: &str) -> (i64, i64) {
    let nt = NewTicket::validated(NewTicketInput {
        email: email.into(),
        name: "Canned Reply Tester".into(),
        subject: "needs a canned reply".into(),
        body: "first message".into(),
        source: Some("Web".into()),
        dept_id: None,
    })
    .unwrap();
    let t = create_ticket(pool, &nt).await.unwrap();
    (t.ticket_id, t.ticket_number)
}

async fn canned_id_by_title(pool: &PgPool, title: &str) -> i32 {
    sqlx::query_scalar("SELECT canned_id FROM canned_response WHERE title = $1")
        .bind(title)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn count_attachment_files(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM attachment_file")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn ticket_detail(router: &Router, id: i64, cookie: &str) -> serde_json::Value {
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/staff/tickets/{id}"))
                .header(COOKIE, format!("ost_staff_sess={cookie}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    json_body(resp).await
}

/// Build a multipart body from (name, value, optional filename) parts.
fn multipart_body(boundary: &str, parts: &[(&str, &[u8], Option<&str>)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (name, value, filename) in parts {
        out.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        match filename {
            Some(fname) => out.extend_from_slice(
                format!(
                    "Content-Disposition: form-data; name=\"{name}\"; filename=\"{fname}\"\r\n\
                     Content-Type: image/png\r\n\r\n"
                )
                .as_bytes(),
            ),
            None => out.extend_from_slice(
                format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
            ),
        }
        out.extend_from_slice(value);
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    out
}

async fn post_multipart(
    router: &Router,
    id: i64,
    cookie: &str,
    csrf: &str,
    parts: &[(&str, &[u8], Option<&str>)],
) -> http::Response<Body> {
    let boundary = "ostbnd123456";
    let body = multipart_body(boundary, parts);
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/staff/tickets/{id}/reply"))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .header(COOKIE, format!("ost_staff_sess={cookie}; XSRF-TOKEN-STAFF={csrf}"))
                .header("X-CSRFToken", csrf)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Find the last R entry in a detail payload.
fn last_r_entry(detail: &serde_json::Value) -> &serde_json::Value {
    detail["entries"]
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .find(|e| e["threadType"] == "R")
        .expect("an R entry exists")
}

// --- AC-1 + AC-2: canned reply posts substituted body + attachments + answered

#[tokio::test]
async fn canned_reply_posts_substituted_body_carries_attachment_marks_answered() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping canned_reply_posts_substituted_body...");
        return;
    };
    let router = dev_app(pool.clone());
    let (id, number) = make_ticket(&pool, "d4-ac1@example.com").await;
    let (cookie, csrf) = login(&router).await;

    // Pre-reply: ticket is unanswered (AC-2 setup).
    let before = ticket_detail(&router, id, &cookie).await;
    assert_eq!(before["isanswered"], false, "ticket starts unanswered");

    let files_before = count_attachment_files(&pool).await;
    let canned_id = canned_id_by_title(&pool, "Sample (with attachment)").await;

    let resp = post_multipart(
        &router,
        id,
        &cookie,
        &csrf,
        &[
            ("cannedId", canned_id.to_string().as_bytes(), None),
            ("body", b"", None),
        ],
    )
    .await;
    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::CREATED,
        "canned reply posts: {}",
        resp.status()
    );
    let detail = json_body(resp).await;

    // AC-2: the ticket is now answered, the R entry's author is the agent.
    assert_eq!(detail["isanswered"], true, "any staff reply marks answered");
    let entry = last_r_entry(&detail);
    assert_eq!(entry["poster"], "Agent One", "author is the posting agent");

    // AC-1: the body is substituted (ticket number present, no literal %{...}).
    let body = entry["body"].as_str().unwrap();
    assert!(
        body.contains(&number.to_string()),
        "substituted body holds the ticket number {number}: {body}"
    );
    assert!(!body.contains("%{"), "no literal %{{...}} remains: {body}");

    // AC-1: the R entry carries policy.txt under the shared attachments key.
    let atts = entry["attachments"].as_array().unwrap();
    assert_eq!(atts.len(), 1, "one carried canned attachment");
    assert_eq!(atts[0]["name"], "policy.txt");

    // AC-1: NO new blob/attachment_file row — the canned blob is re-bound by id.
    assert_eq!(
        count_attachment_files(&pool).await,
        files_before,
        "the canned blob is re-bound by file id, not re-uploaded (D1)"
    );
}

// --- AC-3: own file + canned in one reply both bind ------------------------

#[tokio::test]
async fn own_file_and_canned_both_bind() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping own_file_and_canned_both_bind");
        return;
    };
    let router = dev_app(pool.clone());
    let (id, _) = make_ticket(&pool, "d4-ac3@example.com").await;
    let (cookie, csrf) = login(&router).await;
    let canned_id = canned_id_by_title(&pool, "Sample (with attachment)").await;

    // A tiny payload (allow-list is extension-only; .png permitted, < 1 MiB).
    // Avoid CR/LF bytes so the multipart boundary parsing stays intact.
    let png = b"fake-png-bytes-allowed-by-extension-only";
    let resp = post_multipart(
        &router,
        id,
        &cookie,
        &csrf,
        &[
            ("cannedId", canned_id.to_string().as_bytes(), None),
            ("body", b"", None),
            ("attachment", png, Some("note.png")),
        ],
    )
    .await;
    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::CREATED,
        "own+canned reply posts: {}",
        resp.status()
    );
    let detail = json_body(resp).await;
    let entry = last_r_entry(&detail);
    let names: Vec<String> = entry["attachments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"policy.txt".to_string()), "canned bound: {names:?}");
    assert!(names.contains(&"note.png".to_string()), "own file bound: {names:?}");
}

// --- AC-4: plain JSON reply (no canned, no file) still posts (M1) ----------

#[tokio::test]
async fn plain_reply_still_posts_and_marks_answered() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping plain_reply_still_posts...");
        return;
    };
    let router = dev_app(pool.clone());
    let (id, _) = make_ticket(&pool, "d4-ac4@example.com").await;
    let (cookie, csrf) = login(&router).await;

    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/staff/tickets/{id}/reply"))
                .header("content-type", "application/json")
                .header(
                    COOKIE,
                    format!("ost_staff_sess={cookie}; XSRF-TOKEN-STAFF={csrf}"),
                )
                .header("X-CSRFToken", csrf.clone())
                .body(Body::from(r#"{"body":"plain"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::CREATED,
        "plain reply posts: {}",
        resp.status()
    );
    let detail = json_body(resp).await;
    let entry = last_r_entry(&detail);
    assert_eq!(entry["body"], "plain");
    assert!(
        entry["attachments"].as_array().unwrap().is_empty(),
        "plain reply has no attachments"
    );
    // §3: a plain staff reply marks the ticket answered too.
    assert_eq!(detail["isanswered"], true, "plain reply marks answered");
    assert_eq!(detail["status"], "open", "status unchanged (M1)");
}

// --- disabled cannedId on the reply path ⇒ 404 -----------------------------

#[tokio::test]
async fn disabled_canned_reply_is_404() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping disabled_canned_reply_is_404");
        return;
    };
    let router = dev_app(pool.clone());
    let (id, _) = make_ticket(&pool, "d4-404@example.com").await;
    let (cookie, csrf) = login(&router).await;
    let disabled_id = canned_id_by_title(&pool, "Closed — disabled sample").await;

    let resp = post_multipart(
        &router,
        id,
        &cookie,
        &csrf,
        &[
            ("cannedId", disabled_id.to_string().as_bytes(), None),
            ("body", b"", None),
        ],
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "a disabled cannedId on the reply path is 404 (posts no reply)"
    );
    // And the ticket stays unanswered (no reply was posted).
    let detail = ticket_detail(&router, id, &cookie).await;
    assert_eq!(detail["isanswered"], false, "no reply ⇒ still unanswered");
}
