//! Integration tests for TS-M3-A1: POST /api/dev/seed-tickets endpoint.
//!
//! Tests the dev endpoint for seeding tickets with configurable flags.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test seed_tickets
//! ```
//!
//! @implements TS-M3-A1: seed-tickets dev endpoint.

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
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn seed_tickets_creates_tickets_with_flags() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping seed_tickets_creates_tickets_with_flags");
        return;
    };
    let router = dev_app(pool.clone());

    // Get the agent's staff_id (for potential use in test data)
    let _staff_id: i32 = sqlx::query_scalar("SELECT staff_id FROM staff WHERE username = 'agent'")
        .fetch_one(&pool)
        .await
        .unwrap();

    // Create 3 tickets with different flags
    let payload = serde_json::json!({
        "tickets": [
            {"status": "open", "isanswered": false, "isoverdue": false, "staff_id": null},
            {"status": "open", "isanswered": true, "staff_id": null},
            {"status": "closed"}
        ]
    });

    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/dev/seed-tickets")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    let ticket_ids = body["ticket_ids"].as_array().expect("should have ticket_ids");
    assert_eq!(ticket_ids.len(), 3, "should have created 3 tickets");

    // Verify the flags were set correctly
    let first_id = ticket_ids[0].as_i64().unwrap();
    let (status, isanswered, isoverdue, ticket_staff_id): (String, bool, bool, Option<i32>) =
        sqlx::query_as(
            "SELECT status, isanswered, isoverdue, staff_id FROM ticket WHERE ticket_id = $1",
        )
        .bind(first_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "open");
    assert!(!isanswered);
    assert!(!isoverdue);
    assert!(ticket_staff_id.is_none());

    let second_id = ticket_ids[1].as_i64().unwrap();
    let (status, isanswered, _isoverdue, _ticket_staff_id): (String, bool, bool, Option<i32>) =
        sqlx::query_as(
            "SELECT status, isanswered, isoverdue, staff_id FROM ticket WHERE ticket_id = $1",
        )
        .bind(second_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "open");
    assert!(isanswered, "second ticket should be answered");

    let third_id = ticket_ids[2].as_i64().unwrap();
    let (status, _isanswered, _isoverdue, _ticket_staff_id): (String, bool, bool, Option<i32>) =
        sqlx::query_as(
            "SELECT status, isanswered, isoverdue, staff_id FROM ticket WHERE ticket_id = $1",
        )
        .bind(third_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "closed", "third ticket should be closed");
}

#[tokio::test]
async fn seed_tickets_returns_404_in_production() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping seed_tickets_returns_404_in_production");
        return;
    };
    // Use Production env, not Development
    let router = app(
        AppState::with_pool(pool).with_app_env(AppEnv::Production),
        ORIGIN,
    );

    let payload = serde_json::json!({
        "tickets": [{"status": "open"}]
    });

    let resp = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/dev/seed-tickets")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
