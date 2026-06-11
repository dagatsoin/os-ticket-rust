//! Integration tests for the TS-M2-A5 staff reply multipart attachment hook +
//! the §7 `attachments` array on thread payloads. Driven through the router via
//! `oneshot`; skipped (pass, log line) when `TEST_DATABASE_URL` is unset.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test reply_attachment
//! ```
//!
//! @implements FS-021.3 / FS-021.16: a permitted reply file binds ref_type R
//!   (AC-1); a disallowed/oversize file ⇒ 422, no R entry (AC-2).
//! @implements ROADMAP §7: staff detail + client thread expose each entry's
//!   `attachments: [{id,name,size,mime}]` (AC-3); JSON reply ⇒ `[]` (AC-4).

use api::{app, AppEnv, AppState};
use axum::body::Body;
use axum::Router;
use http::header::{COOKIE, SET_COOKIE};
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use ost_core::ticket::{create_ticket, NewTicket, NewTicketInput};
use ost_core::BlobStore;
use sqlx::postgres::PgPool;
use tower::ServiceExt;

const ORIGIN: &str = "http://localhost:3702";
const BOUNDARY: &str = "----osticketA5boundary";

fn test_db_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

fn temp_store() -> (BlobStore, std::path::PathBuf) {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("ost-a5-test-{nonce}"));
    (BlobStore::new(&root), root)
}

async fn seeded_pool() -> Option<PgPool> {
    let url = test_db_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    tools::seed(&pool).await.expect("seed");
    // The config-key seed is `ON CONFLICT DO NOTHING`, and a sibling test binary
    // (`config_seed`) intentionally mutates `allowed_filetypes`. Force the policy
    // these tests depend on so they are order-independent across the shared DB.
    set_policy(&pool).await;
    Some(pool)
}

/// Upsert the attachment policy this suite asserts against (overwrite, unlike the
/// seed's DO NOTHING), so cross-binary config pollution cannot flake the suite.
async fn set_policy(pool: &PgPool) {
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
        .execute(pool)
        .await
        .unwrap();
    }
}

fn dev_app(pool: PgPool, store: BlobStore) -> Router {
    app(
        AppState::with_pool(pool)
            .with_app_env(AppEnv::Development)
            .with_store(store),
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

fn multipart_body(fields: &[(&str, &str)], file: Option<(&str, &[u8])>) -> Vec<u8> {
    let mut out = Vec::new();
    for (name, value) in fields {
        out.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
        out.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
        );
        out.extend_from_slice(value.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    if let Some((filename, bytes)) = file {
        out.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
        out.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"attachment\"; filename=\"{filename}\"\r\n"
            )
            .as_bytes(),
        );
        out.extend_from_slice(b"Content-Type: image/png\r\n\r\n");
        out.extend_from_slice(bytes);
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    out
}

async fn post_reply_multipart(
    router: &Router,
    id: i64,
    cookie: &str,
    csrf: &str,
    body: Vec<u8>,
) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/staff/tickets/{id}/reply"))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={BOUNDARY}"),
                )
                .header(COOKIE, format!("ost_staff_sess={cookie}; XSRF-TOKEN-STAFF={csrf}"))
                .header("X-CSRFToken", csrf)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap()
}

// --- AC-1 / AC-3: permitted reply file → R binding + attachments in payload. --

#[tokio::test]
async fn reply_with_permitted_file_binds_r_attachment_and_appears_in_detail() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping reply_with_permitted_file...");
        return;
    };
    let (store, root) = temp_store();
    let router = dev_app(pool.clone(), store);
    let id = make_ticket(&pool, "a5-ac1@example.com").await;
    let (cookie, csrf) = login(&router).await;

    let png = b"\x89PNG\r\n fake note image";
    let body = multipart_body(&[("body", "here you go")], Some(("note.png", png)));
    let resp = post_reply_multipart(&router, id, &cookie, &csrf, body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let detail = json_body(resp).await;

    // The latest binding is ref_type R.
    let ref_type: String =
        sqlx::query_scalar("SELECT ref_type FROM ticket_attachment ORDER BY id DESC LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(ref_type, "R");

    // The R entry in the response lists note.png under `attachments`.
    let entries = detail["entries"].as_array().unwrap();
    let r = entries.iter().find(|e| e["threadType"] == "R").unwrap();
    let atts = r["attachments"].as_array().unwrap();
    assert_eq!(atts.len(), 1, "R entry has one attachment");
    assert_eq!(atts[0]["name"], "note.png");
    assert_eq!(atts[0]["size"], png.len() as i64);
    assert_eq!(atts[0]["mime"], "image/png");
    assert!(atts[0]["id"].is_i64());

    // The original M entry has an empty attachments array (AC-3 shape).
    let m = entries.iter().find(|e| e["threadType"] == "M").unwrap();
    assert_eq!(m["attachments"].as_array().unwrap().len(), 0);

    let _ = std::fs::remove_dir_all(&root);
}

// --- AC-2: disallowed / oversize reply file → 422, no R entry. ---------------

#[tokio::test]
async fn disallowed_or_oversize_reply_file_returns_422_and_posts_no_reply() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping disallowed_or_oversize_reply_file...");
        return;
    };
    let (store, root) = temp_store();
    let router = dev_app(pool.clone(), store);
    let id = make_ticket(&pool, "a5-ac2@example.com").await;
    let (cookie, csrf) = login(&router).await;

    let before_r: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ticket_thread WHERE thread_type = 'R'")
            .fetch_one(&pool)
            .await
            .unwrap();

    // Disallowed extension.
    let evil = multipart_body(&[("body", "x")], Some(("evil.exe", b"MZ")));
    let r1 = post_reply_multipart(&router, id, &cookie, &csrf, evil).await;
    assert_eq!(r1.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json_body(r1).await["error"]["fields"]["attachment"].is_string());

    // Oversize.
    let big = vec![0u8; 1_048_576 + 10];
    let big_body = multipart_body(&[("body", "x")], Some(("big.png", &big)));
    let r2 = post_reply_multipart(&router, id, &cookie, &csrf, big_body).await;
    assert_eq!(r2.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json_body(r2).await["error"]["fields"]["attachment"].is_string());

    let after_r: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ticket_thread WHERE thread_type = 'R'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(before_r, after_r, "no R entry posted on a rejected attachment");

    let _ = std::fs::remove_dir_all(&root);
}

// --- AC-4: JSON reply (no attachment) → 200 + empty attachments. -------------

#[tokio::test]
async fn json_reply_returns_empty_attachments_array() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping json_reply_returns_empty_attachments...");
        return;
    };
    let (store, root) = temp_store();
    let router = dev_app(pool.clone(), store);
    let id = make_ticket(&pool, "a5-ac4@example.com").await;
    let (cookie, csrf) = login(&router).await;

    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/staff/tickets/{id}/reply"))
                .header("content-type", "application/json")
                .header(COOKIE, format!("ost_staff_sess={cookie}; XSRF-TOKEN-STAFF={csrf}"))
                .header("X-CSRFToken", &csrf)
                .body(Body::from(r#"{"body":"plain reply"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let detail = json_body(resp).await;
    let r = detail["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["threadType"] == "R")
        .unwrap();
    assert_eq!(
        r["attachments"].as_array().unwrap().len(),
        0,
        "JSON reply has an empty attachments array (M1 contract)"
    );

    let _ = std::fs::remove_dir_all(&root);
}
