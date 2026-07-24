//! Integration tests for the TS-M2-A3 public-create multipart attachment hook.
//! Driven through the router via `oneshot` against a live Postgres test DB;
//! skipped (pass, log line) when `TEST_DATABASE_URL` is unset.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test create_attachment
//! ```
//!
//! @implements FS-011.7: create with a permitted file binds a ticket_attachment
//!   (ref_type M) to the M entry (AC-1); identical bytes dedup (AC-3).
//! @implements EC-011.5: a disallowed/oversized file ⇒ 422, no ticket, no blob
//!   (AC-2). A JSON no-attachment create stays 201 (AC-4, M1 contract).

use api::{app, AppEnv, AppState};
use axum::body::Body;
use axum::Router;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use ost_core::BlobStore;
use sqlx::postgres::PgPool;
use tower::ServiceExt;

const ORIGIN: &str = "http://localhost:3702";
const BOUNDARY: &str = "----osticketTESTboundary";

fn test_db_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

/// A throwaway blob store rooted under the OS temp dir, unique per test.
fn temp_store() -> (BlobStore, std::path::PathBuf) {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("ost-a3-test-{nonce}"));
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

async fn json_body(resp: http::Response<Body>) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

/// Build a multipart/form-data body from text fields + an optional file part.
fn multipart_body(
    fields: &[(&str, &str)],
    file: Option<(&str, &str, &[u8])>, // (field, filename, bytes)
) -> Vec<u8> {
    let mut out = Vec::new();
    for (name, value) in fields {
        out.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
        out.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
        );
        out.extend_from_slice(value.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    if let Some((field, filename, bytes)) = file {
        out.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
        out.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{field}\"; filename=\"{filename}\"\r\n"
            )
            .as_bytes(),
        );
        out.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
        out.extend_from_slice(bytes);
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    out
}

async fn post_multipart(router: &Router, body: Vec<u8>) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tickets")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={BOUNDARY}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap()
}

// --- AC-1 / AC-3: permitted file → 201 + ref_type M binding; dedup. ----------

#[tokio::test]
async fn create_with_permitted_file_binds_m_attachment_and_dedups() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping create_with_permitted_file...");
        return;
    };
    let (store, root) = temp_store();
    let router = dev_app(pool.clone(), store);

    let pdf = b"%PDF-1.4 fake invoice bytes";
    let body = multipart_body(
        &[
            ("name", "Mia"),
            ("email", "mia@example.com"),
            ("subject", "Inv"),
            ("message", "see attached"),
        ],
        Some(("attachment", "invoice.pdf", pdf)),
    );
    let resp = post_multipart(&router, body).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let number1 = json_body(resp).await["ticketNumber"].as_i64().unwrap();
    assert!((100_000..=999_999).contains(&number1));

    // The M entry has a ticket_attachment (ref_type M) naming invoice.pdf.
    let row: (String, String, i64) = sqlx::query_as(
        r#"SELECT ta.ref_type, af.name, af.size
           FROM ticket_attachment ta
           JOIN attachment_file af ON af.id = ta.file_id
           JOIN ticket t ON t.ticket_id = ta.ticket_id
           WHERE t."ticketID" = $1"#,
    )
    .bind(number1)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.0, "M", "bound to the M entry");
    assert_eq!(row.1, "invoice.pdf");
    assert_eq!(row.2, pdf.len() as i64);

    // AC-3: a second create with the SAME bytes → 2 bindings, 1 attachment_file.
    let body2 = multipart_body(
        &[
            ("name", "Mia"),
            ("email", "mia2@example.com"),
            ("subject", "Inv2"),
            ("message", "again"),
        ],
        Some(("attachment", "invoice.pdf", pdf)),
    );
    let resp2 = post_multipart(&router, body2).await;
    assert_eq!(resp2.status(), StatusCode::CREATED);
    let number2 = json_body(resp2).await["ticketNumber"].as_i64().unwrap();
    assert_ne!(number1, number2);

    let hash = ost_core::sha256_hex(pdf);
    let file_rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM attachment_file WHERE hash = $1")
            .bind(&hash)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(file_rows, 1, "identical bytes share one attachment_file (D1)");

    // Exactly one physical blob exists for that hash.
    assert!(
        ost_core::BlobStore::new(&root).exists(&hash).await.unwrap(),
        "blob present on disk"
    );

    let _ = std::fs::remove_dir_all(&root);
}

// --- AC-2: disallowed / oversized → 422, no ticket, no blob. -----------------

#[tokio::test]
async fn disallowed_or_oversize_file_returns_422_and_creates_nothing() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping disallowed_or_oversize_file...");
        return;
    };
    let (store, root) = temp_store();
    let router = dev_app(pool.clone(), store);

    let before_tickets: i64 = sqlx::query_scalar("SELECT count(*) FROM ticket")
        .fetch_one(&pool)
        .await
        .unwrap();
    let before_files: i64 = sqlx::query_scalar("SELECT count(*) FROM attachment_file")
        .fetch_one(&pool)
        .await
        .unwrap();

    // Disallowed extension (.exe is not in the seeded allow-list).
    let evil = multipart_body(
        &[
            ("name", "Mia"),
            ("email", "mia@example.com"),
            ("subject", "Bad"),
            ("message", "x"),
        ],
        Some(("attachment", "evil.exe", b"MZ malware")),
    );
    let r1 = post_multipart(&router, evil).await;
    assert_eq!(r1.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let j1 = json_body(r1).await;
    assert!(
        j1["error"]["fields"]["attachment"].is_string(),
        "file error keyed on `attachment`"
    );

    // Oversized: > seeded max_file_size (1 MiB).
    let big = vec![0u8; 1_048_576 + 10];
    let big_body = multipart_body(
        &[
            ("name", "Mia"),
            ("email", "mia@example.com"),
            ("subject", "Big"),
            ("message", "x"),
        ],
        Some(("attachment", "big.pdf", &big)),
    );
    let r2 = post_multipart(&router, big_body).await;
    assert_eq!(r2.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json_body(r2).await["error"]["fields"]["attachment"].is_string());

    // No ticket, no attachment_file row created.
    let after_tickets: i64 = sqlx::query_scalar("SELECT count(*) FROM ticket")
        .fetch_one(&pool)
        .await
        .unwrap();
    let after_files: i64 = sqlx::query_scalar("SELECT count(*) FROM attachment_file")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before_tickets, after_tickets, "no ticket on rejected upload");
    assert_eq!(before_files, after_files, "no attachment_file on rejected upload");

    let _ = std::fs::remove_dir_all(&root);
}

// --- AC-4: JSON (no attachment) still 201 + no ticket_attachment. ------------

#[tokio::test]
async fn json_create_without_attachment_still_201() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping json_create_without_attachment...");
        return;
    };
    let (store, root) = temp_store();
    let router = dev_app(pool.clone(), store);

    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tickets")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Mia","email":"plain@example.com","subject":"Plain","message":"hi"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let number = json_body(resp).await["ticketNumber"].as_i64().unwrap();

    let bindings: i64 = sqlx::query_scalar(
        r#"SELECT count(*) FROM ticket_attachment ta
           JOIN ticket t ON t.ticket_id = ta.ticket_id
           WHERE t."ticketID" = $1"#,
    )
    .bind(number)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(bindings, 0, "JSON create has no attachments (M1 contract)");

    let _ = std::fs::remove_dir_all(&root);
}
