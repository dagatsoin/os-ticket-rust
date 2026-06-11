//! Integration tests for the TS-M1-B2 public create-ticket route. Driven through
//! the router via `oneshot` against a live Postgres test DB (schema + seed
//! applied); skipped (pass, with a log line) when `TEST_DATABASE_URL` is unset so
//! offline / DB-less CI stays green:
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test tickets
//! ```
//!
//! @implements BS-011: 201 + ticket number on valid payload (AC-1); 422 + field
//!   map on invalid payload (AC-2).
//! @implements FS-003.10: HTML sanitisation before persist (AC-3).

use api::{app, AppEnv, AppState};
use axum::body::Body;
use axum::Router;
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

async fn json_body(resp: http::Response<Body>) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

async fn post_ticket(router: &Router, body: &str) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tickets")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Count thread bodies persisted for a given external ticket number.
async fn loaded_body_for_number(pool: &PgPool, number: i64) -> Option<String> {
    let row: Option<(String,)> = sqlx::query_as(
        r#"SELECT tt.body FROM ticket_thread tt
           JOIN ticket t ON t.ticket_id = tt.ticket_id
           WHERE t."ticketID" = $1 AND tt.thread_type = 'M'"#,
    )
    .bind(number)
    .fetch_optional(pool)
    .await
    .unwrap();
    row.map(|(b,)| b)
}

// --- AC-1: valid payload → 201 + 6-digit number + persisted -----------------

#[tokio::test]
async fn valid_payload_returns_201_with_six_digit_number_and_persists() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping valid_payload_returns_201...");
        return;
    };
    let router = dev_app(pool.clone());

    let resp = post_ticket(
        &router,
        r#"{"name":"Jane Doe","email":"jane@example.com","subject":"Printer broken","message":"It won't print."}"#,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = json_body(resp).await;
    let number = json["ticketNumber"].as_i64().expect("ticketNumber present");
    assert!((100_000..=999_999).contains(&number), "6-digit number: {number}");

    // Persisted: the first M entry exists for that number.
    assert!(loaded_body_for_number(&pool, number).await.is_some());
}

// --- AC-2: missing/invalid → 422 + field map, no ticket created -------------

#[tokio::test]
async fn invalid_payload_returns_422_with_field_errors_and_creates_no_ticket() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping invalid_payload_returns_422...");
        return;
    };
    let router = dev_app(pool.clone());

    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM ticket")
        .fetch_one(&pool)
        .await
        .unwrap();

    let resp = post_ticket(
        &router,
        r#"{"name":"Jane","email":"not-an-email","subject":"","message":""}"#,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let json = json_body(resp).await;
    assert!(json["error"]["message"].is_string());
    let fields = &json["error"]["fields"];
    assert!(fields["email"].is_string(), "email error present");
    assert!(fields["subject"].is_string(), "subject error present");
    // The public `message` key (not the internal `body`) carries the error.
    assert!(fields["message"].is_string(), "message error present");
    assert!(fields["body"].is_null(), "internal `body` key must not leak");

    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM ticket")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, after, "no ticket created on validation failure");
}

// --- AC-3: HTML in name/subject/message is sanitised before persist ---------

#[tokio::test]
async fn html_is_sanitised_before_persist() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping html_is_sanitised_before_persist");
        return;
    };
    let router = dev_app(pool.clone());

    let resp = post_ticket(
        &router,
        r#"{"name":"<b>Jane</b>","email":"xss@example.com","subject":"<script>alert(1)</script>Help","message":"<script>alert(1)</script>hello"}"#,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let number = json_body(resp).await["ticketNumber"].as_i64().unwrap();

    let body = loaded_body_for_number(&pool, number).await.unwrap();
    assert!(!body.to_lowercase().contains("<script"), "script stripped: {body}");
    assert!(body.contains("hello"), "safe text preserved");

    let subject: String =
        sqlx::query_scalar(r#"SELECT subject FROM ticket WHERE "ticketID" = $1"#)
            .bind(number)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!subject.to_lowercase().contains("<script"), "subject sanitised: {subject}");
}

// --- AC-4: payload without topicId → 201 (topicId not required in M1) --------

#[tokio::test]
async fn payload_without_topic_id_is_accepted() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping payload_without_topic_id_is_accepted");
        return;
    };
    let router = dev_app(pool);
    // No topicId key anywhere in the body.
    let resp = post_ticket(
        &router,
        r#"{"name":"No Topic","email":"notopic@example.com","subject":"Hi","message":"Body here"}"#,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED, "topicId not required in M1");
}

// --- AC-5: no CSRF token required (public unauthenticated endpoint) ----------

#[tokio::test]
async fn no_csrf_token_required() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping no_csrf_token_required");
        return;
    };
    let router = dev_app(pool);
    // No X-CSRFToken header, no cookie — must still succeed.
    let resp = post_ticket(
        &router,
        r#"{"name":"Anon","email":"anon@example.com","subject":"No CSRF","message":"works"}"#,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED, "POST /api/tickets is CSRF-exempt");
}
