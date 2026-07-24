//! Integration tests for TS-M3-B3 — pagination with limit param + total count.
//!
//! These tests verify that:
//! 1. Route returns pagination metadata (AC-1)
//! 2. Default page size is 25 (AC-2)
//! 3. p param selects the page (AC-3)
//! 4. limit param overrides page size (AC-4)
//! 5. limit is clamped to min 5 (AC-5)
//! 6. limit is clamped to max 100 (AC-6)
//! 9. Page beyond total pages returns empty (AC-9)
//! 10. Invalid p param defaults to page 1 (AC-10)
//! 11. p=0 or negative defaults to page 1 (AC-11)
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test pagination -- --test-threads=1
//! ```
//!
//! @implements TS-M3-B3: pagination with limit/page params + total count.

use api::{app, AppEnv, AppState};
use axum::body::Body;
use axum::Router;
use http::header::SET_COOKIE;
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

async fn reset_pool(pool: &PgPool) {
    tools::reset(pool).await.expect("reset");
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
    format!(
        "ost_staff_sess={}",
        set_cookie_value(&login, "ost_staff_sess").unwrap()
    )
}

async fn make_ticket(pool: &PgPool, subject: &str) -> i64 {
    let nt = NewTicket::validated(NewTicketInput {
        email: "pagination@example.com".into(),
        name: "Pagination Tester".into(),
        subject: subject.into(),
        body: "test body".into(),
        source: Some("Web".into()),
        dept_id: None,
    })
    .unwrap();
    create_ticket(pool, &nt).await.unwrap().ticket_id
}

async fn get(router: &Router, uri: &str, cookie: &str) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .header("Cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

/// AC-1: route returns pagination metadata.
#[tokio::test]
async fn route_returns_pagination_metadata() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping");
        return;
    };
    reset_pool(&pool).await;
    make_ticket(&pool, "PAGINATION_TEST").await;

    let router = dev_app(pool);
    let cookie = staff_cookie(&router).await;
    let resp = get(&router, "/api/staff/tickets?status=open", &cookie).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;

    // Check pagination object exists with required keys.
    assert!(body["pagination"].is_object(), "should have pagination object");
    assert!(body["pagination"]["page"].is_number(), "should have page");
    assert!(body["pagination"]["pageSize"].is_number(), "should have pageSize");
    assert!(body["pagination"]["totalCount"].is_number(), "should have totalCount");
    assert!(body["pagination"]["totalPages"].is_number(), "should have totalPages");
}

/// AC-2: default page size is 25 when no config or param.
#[tokio::test]
async fn default_page_size_is_25() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping");
        return;
    };
    reset_pool(&pool).await;

    // Create 30 tickets.
    for i in 0..30 {
        make_ticket(&pool, &format!("PAGE_SIZE_TEST_{i}")).await;
    }

    let router = dev_app(pool);
    let cookie = staff_cookie(&router).await;
    let resp = get(&router, "/api/staff/tickets?status=open", &cookie).await;

    let body = json_body(resp).await;
    assert_eq!(body["pagination"]["pageSize"].as_i64(), Some(25), "default page size should be 25");
    assert_eq!(body["tickets"].as_array().unwrap().len(), 25, "should return 25 tickets");
    assert_eq!(body["pagination"]["totalCount"].as_i64(), Some(30), "total should be 30");
    assert_eq!(body["pagination"]["totalPages"].as_i64(), Some(2), "should have 2 pages");
}

/// AC-3: p param selects the page.
#[tokio::test]
async fn p_param_selects_page() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping");
        return;
    };
    reset_pool(&pool).await;

    // Create 30 tickets.
    for i in 0..30 {
        make_ticket(&pool, &format!("PAGE_SELECT_TEST_{i}")).await;
    }

    let router = dev_app(pool);
    let cookie = staff_cookie(&router).await;

    // Request page 2.
    let resp = get(&router, "/api/staff/tickets?status=open&p=2", &cookie).await;
    let body = json_body(resp).await;

    assert_eq!(body["pagination"]["page"].as_i64(), Some(2), "should be page 2");
    // With 30 tickets and page size 25, page 2 should have 5 tickets.
    assert_eq!(body["tickets"].as_array().unwrap().len(), 5, "page 2 should have 5 remaining tickets");
}

/// AC-4: limit param overrides page size.
#[tokio::test]
async fn limit_param_overrides_page_size() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping");
        return;
    };
    reset_pool(&pool).await;

    // Create 30 tickets.
    for i in 0..30 {
        make_ticket(&pool, &format!("LIMIT_OVERRIDE_{i}")).await;
    }

    let router = dev_app(pool);
    let cookie = staff_cookie(&router).await;

    let resp = get(&router, "/api/staff/tickets?status=open&limit=10", &cookie).await;
    let body = json_body(resp).await;

    assert_eq!(body["pagination"]["pageSize"].as_i64(), Some(10), "page size should be 10");
    assert_eq!(body["tickets"].as_array().unwrap().len(), 10, "should return 10 tickets");
    assert_eq!(body["pagination"]["totalPages"].as_i64(), Some(3), "should have 3 pages");
}

/// AC-5: limit is clamped to min 5.
#[tokio::test]
async fn limit_clamped_to_min_5() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping");
        return;
    };
    let router = dev_app(pool);
    let cookie = staff_cookie(&router).await;

    let resp = get(&router, "/api/staff/tickets?status=open&limit=2", &cookie).await;
    let body = json_body(resp).await;

    assert_eq!(body["pagination"]["pageSize"].as_i64(), Some(5), "limit should be clamped to 5");
}

/// AC-6: limit is clamped to max 100.
#[tokio::test]
async fn limit_clamped_to_max_100() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping");
        return;
    };
    let router = dev_app(pool);
    let cookie = staff_cookie(&router).await;

    let resp = get(&router, "/api/staff/tickets?status=open&limit=500", &cookie).await;
    let body = json_body(resp).await;

    assert_eq!(body["pagination"]["pageSize"].as_i64(), Some(100), "limit should be clamped to 100");
}

/// AC-9: page beyond total pages returns empty tickets array.
#[tokio::test]
async fn page_beyond_total_returns_empty() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping");
        return;
    };
    reset_pool(&pool).await;

    // Create 10 tickets — with page size 25, only 1 page exists.
    for i in 0..10 {
        make_ticket(&pool, &format!("BEYOND_PAGE_{i}")).await;
    }

    let router = dev_app(pool);
    let cookie = staff_cookie(&router).await;

    let resp = get(&router, "/api/staff/tickets?status=open&p=5", &cookie).await;
    let body = json_body(resp).await;

    assert_eq!(body["pagination"]["page"].as_i64(), Some(5), "should report page 5");
    assert_eq!(body["tickets"].as_array().unwrap().len(), 0, "should return empty tickets array");
    assert_eq!(body["pagination"]["totalPages"].as_i64(), Some(1), "should have only 1 page");
}

/// AC-10: invalid p param defaults to page 1.
#[tokio::test]
async fn invalid_p_defaults_to_page_1() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping");
        return;
    };
    let router = dev_app(pool);
    let cookie = staff_cookie(&router).await;

    let resp = get(&router, "/api/staff/tickets?status=open&p=abc", &cookie).await;
    let body = json_body(resp).await;

    assert_eq!(body["pagination"]["page"].as_i64(), Some(1), "invalid p should default to page 1");
}

/// AC-11: p=0 or negative defaults to page 1.
#[tokio::test]
async fn zero_or_negative_p_defaults_to_page_1() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping");
        return;
    };
    let router = dev_app(pool);
    let cookie = staff_cookie(&router).await;

    // p=0
    let resp = get(&router, "/api/staff/tickets?status=open&p=0", &cookie).await;
    let body = json_body(resp).await;
    assert_eq!(body["pagination"]["page"].as_i64(), Some(1), "p=0 should default to page 1");

    // p=-3
    let resp = get(&router, "/api/staff/tickets?status=open&p=-3", &cookie).await;
    let body = json_body(resp).await;
    assert_eq!(body["pagination"]["page"].as_i64(), Some(1), "p=-3 should default to page 1");
}
