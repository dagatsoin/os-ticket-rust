//! Integration tests for the TS-M2-B1 attachment download routes. Driven through
//! the router via `oneshot`; skipped (pass, log line) when `TEST_DATABASE_URL`
//! is unset.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test download_routes
//! ```
//!
//! @implements FS-022.10 / FS-022.11: an authorized client/staff session streams
//!   the blob with a Content-Disposition filename (AC-1, AC-4).
//! @implements BS-022.8 / EC-022.7 / EC-022.9 (D2, §8): a cross-ticket session
//!   and an unknown id both 404 with no body — indistinguishable, no existence
//!   leak (AC-2, AC-3).

use api::{app, AppEnv, AppState};
use axum::body::Body;
use axum::Router;
use http::header::{COOKIE, SET_COOKIE};
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use ost_core::BlobStore;
use sqlx::postgres::PgPool;
use tower::ServiceExt;

const ORIGIN: &str = "http://localhost:3702";
const BOUNDARY: &str = "----osticketB1boundary";

fn test_db_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

fn temp_store() -> (BlobStore, std::path::PathBuf) {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("ost-b1-test-{nonce}"));
    (BlobStore::new(&root), root)
}

async fn seeded_pool() -> Option<PgPool> {
    let url = test_db_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    tools::seed(&pool).await.expect("seed");
    set_policy(&pool).await;
    Some(pool)
}

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

async fn raw_body(resp: http::Response<Body>) -> Vec<u8> {
    resp.into_body().collect().await.unwrap().to_bytes().to_vec()
}

fn multipart_create(email: &str, filename: &str, bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for (name, value) in [
        ("name", "Mia"),
        ("email", email),
        ("subject", "Inv"),
        ("message", "see attached"),
    ] {
        out.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
        out.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
        );
        out.extend_from_slice(value.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
    out.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"attachment\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    out.extend_from_slice(b"Content-Type: application/pdf\r\n\r\n");
    out.extend_from_slice(bytes);
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    out
}

async fn create_ticket_with_file(
    router: &Router,
    email: &str,
    filename: &str,
    bytes: &[u8],
) -> i64 {
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tickets")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={BOUNDARY}"),
                )
                .body(Body::from(multipart_create(email, filename, bytes)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await["ticketNumber"].as_i64().unwrap()
}

async fn create_plain_ticket(router: &Router, email: &str) -> i64 {
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tickets")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"name":"Bob","email":"{email}","subject":"Plain","message":"hi"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await["ticketNumber"].as_i64().unwrap()
}

async fn staff_login(router: &Router) -> (String, String) {
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

async fn client_login(router: &Router, number: i64, email: &str) -> String {
    let resp = router
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
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "client login");
    set_cookie_value(&resp, "ost_client_sess").unwrap()
}

/// Resolve ticket-A's internal id + the attachmentId of its M-entry file via the
/// staff detail route.
async fn attachment_id_via_staff(
    router: &Router,
    staff_cookie: &str,
    number: i64,
    pool: &PgPool,
) -> (i64, i64) {
    let ticket_id: i64 =
        sqlx::query_scalar(r#"SELECT ticket_id FROM ticket WHERE "ticketID" = $1"#)
            .bind(number)
            .fetch_one(pool)
            .await
            .unwrap();
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/staff/tickets/{ticket_id}"))
                .header(COOKIE, format!("ost_staff_sess={staff_cookie}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let detail = json_body(resp).await;
    let entries = detail["entries"].as_array().unwrap();
    let m = entries.iter().find(|e| e["threadType"] == "M").unwrap();
    let att_id = m["attachments"][0]["id"].as_i64().unwrap();
    (ticket_id, att_id)
}

// --- AC-1 / AC-2 / AC-3: client downloads, cross-ticket + unknown 404. -------

#[tokio::test]
async fn client_download_authorized_and_cross_ticket_and_unknown() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping client_download_authorized...");
        return;
    };
    let (store, root) = temp_store();
    let router = dev_app(pool.clone(), store);

    let pdf = b"%PDF-1.4 the real invoice bytes for B1";
    let a_email = format!("a-{}@example.com", std::process::id());
    let b_email = format!("b-{}@example.com", std::process::id());
    let number_a = create_ticket_with_file(&router, &a_email, "invoice.pdf", pdf).await;
    let number_b = create_plain_ticket(&router, &b_email).await;

    let (staff_cookie, _csrf) = staff_login(&router).await;
    let (_ticket_a_id, att_id) =
        attachment_id_via_staff(&router, &staff_cookie, number_a, &pool).await;

    let client_a = client_login(&router, number_a, &a_email).await;
    let client_b = client_login(&router, number_b, &b_email).await;

    // AC-1: client-A downloads → 200, Content-Disposition filename, bytes match.
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/client/ticket/attachments/{att_id}"))
                .header(COOKIE, format!("ost_client_sess={client_a}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let cd = resp
        .headers()
        .get("content-disposition")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(cd, "attachment; filename=\"invoice.pdf\"");
    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(ct, "application/pdf", "stored MIME");
    assert_eq!(raw_body(resp).await, pdf, "byte-identical download");

    // AC-2: client-B (different ticket) → 404, empty body (no existence leak).
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/client/ticket/attachments/{att_id}"))
                .header(COOKIE, format!("ost_client_sess={client_b}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let cross_body = raw_body(resp).await;

    // AC-3: unknown id (client-A) → 404, same envelope shape — indistinguishable.
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/client/ticket/attachments/999999")
                .header(COOKIE, format!("ost_client_sess={client_a}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let unknown_body = raw_body(resp).await;
    assert_eq!(
        cross_body, unknown_body,
        "cross-ticket and unknown 404s must be indistinguishable (§8)"
    );

    let _ = std::fs::remove_dir_all(&root);
}

// --- AC-4: staff download. ----------------------------------------------------

#[tokio::test]
async fn staff_download_authorized_and_wrong_ticket_404() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping staff_download_authorized...");
        return;
    };
    let (store, root) = temp_store();
    let router = dev_app(pool.clone(), store);

    let pdf = b"%PDF-1.4 staff route bytes";
    let a_email = format!("sa-{}@example.com", std::process::id());
    let number_a = create_ticket_with_file(&router, &a_email, "invoice.pdf", pdf).await;
    let number_other = create_plain_ticket(&router, &format!("so-{}@example.com", std::process::id())).await;

    let (staff_cookie, _csrf) = staff_login(&router).await;
    let (ticket_a_id, att_id) =
        attachment_id_via_staff(&router, &staff_cookie, number_a, &pool).await;
    let other_id: i64 =
        sqlx::query_scalar(r#"SELECT ticket_id FROM ticket WHERE "ticketID" = $1"#)
            .bind(number_other)
            .fetch_one(&pool)
            .await
            .unwrap();

    // AC-4: staff downloads via the staff route → 200, bytes match, filename.
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/staff/tickets/{ticket_a_id}/attachments/{att_id}"))
                .header(COOKIE, format!("ost_staff_sess={staff_cookie}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("content-disposition").unwrap().to_str().unwrap(),
        "attachment; filename=\"invoice.pdf\""
    );
    assert_eq!(raw_body(resp).await, pdf);

    // The same attachmentId under a DIFFERENT ticketId → 404 (parent gate).
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/staff/tickets/{other_id}/attachments/{att_id}"))
                .header(COOKIE, format!("ost_staff_sess={staff_cookie}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND, "wrong ticketId ⇒ 404");

    // No session → 401 (realm gate runs before resolution).
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/staff/tickets/{ticket_a_id}/attachments/{att_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let _ = std::fs::remove_dir_all(&root);
}

// --- Client never downloads an N-note attachment (404, no notes leak). --------

#[tokio::test]
async fn client_cannot_download_an_n_note_attachment() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping client_cannot_download_an_n_note...");
        return;
    };
    let (store, root) = temp_store();
    let router = dev_app(pool.clone(), store.clone());

    let pdf = b"%PDF-1.4 note-only bytes for the N check";
    let email = format!("nn-{}@example.com", std::process::id());
    let number = create_plain_ticket(&router, &email).await;
    let ticket_id: i64 =
        sqlx::query_scalar(r#"SELECT ticket_id FROM ticket WHERE "ticketID" = $1"#)
            .bind(number)
            .fetch_one(&pool)
            .await
            .unwrap();

    // Bind an attachment to an N (internal note) entry directly (the API never
    // exposes this binding to a client). Put the blob first so a download could
    // otherwise stream it.
    let hash = store.put(pdf).await.unwrap();
    let storage_key = format!("{}/{}/{}", &hash[0..2], &hash[2..4], hash);
    let note_id: i64 = sqlx::query_scalar(
        "INSERT INTO ticket_thread (ticket_id, thread_type, body) VALUES ($1, 'N', 'note') RETURNING id",
    )
    .bind(ticket_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    // Upsert by hash (the test DB persists across runs, so identical bytes may
    // already have a row — D1 dedup).
    let file_id: i64 = sqlx::query_scalar(
        "INSERT INTO attachment_file (mime, size, hash, name, storage_key)
         VALUES ('application/pdf', $1, $2, 'secret-note.pdf', $3)
         ON CONFLICT (hash) DO UPDATE SET name = attachment_file.name RETURNING id",
    )
    .bind(pdf.len() as i64)
    .bind(&hash)
    .bind(&storage_key)
    .fetch_one(&pool)
    .await
    .unwrap();
    let att_id: i64 = sqlx::query_scalar(
        "INSERT INTO ticket_attachment (ticket_id, file_id, ref_id, ref_type)
         VALUES ($1, $2, $3, 'N') RETURNING id",
    )
    .bind(ticket_id)
    .bind(file_id)
    .bind(note_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    // The client owns the parent ticket, but the attachment hangs off an N entry
    // ⇒ 404 (a client never sees internal notes), no existence leak.
    let client = client_login(&router, number, &email).await;
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/client/ticket/attachments/{att_id}"))
                .header(COOKIE, format!("ost_client_sess={client}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND, "N-note attachment hidden from client");

    // …but a staff session on the parent ticket CAN download it.
    let (staff_cookie, _csrf) = staff_login(&router).await;
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/staff/tickets/{ticket_id}/attachments/{att_id}"))
                .header(COOKIE, format!("ost_staff_sess={staff_cookie}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "staff may download an N-note attachment");

    let _ = std::fs::remove_dir_all(&root);
}
