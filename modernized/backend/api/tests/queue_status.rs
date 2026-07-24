//! Integration tests for TS-M3-A1: BS-020.1 status param handling + quick-stats endpoint.
//!
//! Tests the `GET /api/staff/tickets?status={open|answered|assigned|overdue|closed}` filtering
//! and the `GET /api/staff/tickets/stats` quick-counts endpoint. Driven through the router
//! via `oneshot`; skipped (pass, log line) when `TEST_DATABASE_URL` is unset.
//!
//! Tests use unique ticket subjects to identify test-created tickets and verify they are
//! correctly included/excluded from queue results. Tests run sequentially to avoid race
//! conditions on the shared test database.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test queue_status -- --test-threads=1
//! ```
//!
//! @implements BS-020.1: status param handling (queue selection).
//! @implements FS-020.11: quick-stats endpoint.

use api::{app, AppEnv, AppState};
use axum::body::Body;
use axum::Router;
use http::header::{COOKIE, SET_COOKIE};
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use ost_core::ticket::{create_ticket, NewTicket, NewTicketInput};
use sqlx::postgres::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

const ORIGIN: &str = "http://localhost:3702";

/// Global counter for unique test identifiers.
static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique test prefix for this test run.
fn unique_prefix() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let count = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("TEST_{ts}_{count}")
}

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

/// Reset the pool to a clean state (purge all tickets).
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

/// Create an open ticket with the given subject; returns its internal id.
async fn make_ticket(pool: &PgPool, subject: &str) -> i64 {
    let nt = NewTicket::validated(NewTicketInput {
        email: "queue@example.com".into(),
        name: "Queue Tester".into(),
        subject: subject.into(),
        body: "first message".into(),
        source: Some("Web".into()),
        dept_id: None,
    })
    .unwrap();
    create_ticket(pool, &nt).await.unwrap().ticket_id
}

/// Create a ticket with specific flags set.
async fn make_ticket_with_flags(
    pool: &PgPool,
    subject: &str,
    status: &str,
    isanswered: bool,
    isoverdue: bool,
    staff_id: Option<i32>,
) -> i64 {
    let id = make_ticket(pool, subject).await;
    sqlx::query(
        "UPDATE ticket SET status = $1, isanswered = $2, isoverdue = $3, staff_id = $4 WHERE ticket_id = $5",
    )
    .bind(status)
    .bind(isanswered)
    .bind(isoverdue)
    .bind(staff_id)
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn get(router: &Router, uri: &str, cookie: Option<&str>) -> http::Response<Body> {
    let mut b = Request::builder().uri(uri);
    if let Some(c) = cookie {
        b = b.header(COOKIE, c);
    }
    router
        .clone()
        .oneshot(b.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

/// Get the staff_id for the seeded agent.
async fn get_agent_staff_id(pool: &PgPool) -> i32 {
    sqlx::query_scalar("SELECT staff_id FROM staff WHERE username = 'agent'")
        .fetch_one(pool)
        .await
        .unwrap()
}

// --- AC-1: BS-020.1 — status=open returns only open, non-answered, unassigned (toggles off) ---

#[tokio::test]
async fn status_open_filters_correctly_when_toggles_off() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping status_open_filters_correctly_when_toggles_off");
        return;
    };
    reset_pool(&pool).await;

    // Disable the show_assigned and show_answered toggles
    sqlx::query("UPDATE config SET value = '0' WHERE key IN ('show_assigned_tickets', 'show_answered_tickets')")
        .execute(&pool)
        .await
        .unwrap();

    let prefix = unique_prefix();
    let staff_id = get_agent_staff_id(&pool).await;

    // Create test tickets: one open/unanswered/unassigned, one open/answered, one open/assigned
    let open_id = make_ticket_with_flags(&pool, &format!("{prefix}_OPEN"), "open", false, false, None).await;
    let answered_id =
        make_ticket_with_flags(&pool, &format!("{prefix}_ANSWERED"), "open", true, false, None).await;
    let assigned_id =
        make_ticket_with_flags(&pool, &format!("{prefix}_ASSIGNED"), "open", false, false, Some(staff_id))
            .await;

    let router = dev_app(pool.clone());
    let cookie = staff_cookie(&router).await;
    let resp = get(&router, "/api/staff/tickets?status=open", Some(&cookie)).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let items = body["tickets"].as_array().expect("tickets should be an array");

    // Extract IDs from items
    let ids: Vec<i64> = items.iter().filter_map(|i| i["id"].as_i64()).collect();

    // When toggles are off:
    // - open_id should be present (it's open, not answered, not assigned, not overdue)
    // - answered_id should be excluded (isanswered = true)
    // - assigned_id should be excluded (staff_id is set)
    assert!(ids.contains(&open_id), "open ticket should be in open queue");
    assert!(!ids.contains(&answered_id), "answered ticket should be excluded from open queue when toggle off");
    assert!(!ids.contains(&assigned_id), "assigned ticket should be excluded from open queue when toggle off");
}

// --- AC-2: BS-020.1 — status=answered returns open+answered tickets ---

#[tokio::test]
async fn status_answered_returns_open_answered_tickets() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping status_answered_returns_open_answered_tickets");
        return;
    };
    reset_pool(&pool).await;

    let prefix = unique_prefix();

    // Create test tickets
    let open_id = make_ticket_with_flags(&pool, &format!("{prefix}_OPEN"), "open", false, false, None).await;
    let answered_id =
        make_ticket_with_flags(&pool, &format!("{prefix}_ANSWERED"), "open", true, false, None).await;
    let closed_id =
        make_ticket_with_flags(&pool, &format!("{prefix}_CLOSED"), "closed", false, false, None).await;

    let router = dev_app(pool.clone());
    let cookie = staff_cookie(&router).await;
    let resp = get(&router, "/api/staff/tickets?status=answered", Some(&cookie)).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let items = body["tickets"].as_array().expect("tickets should be an array");

    let ids: Vec<i64> = items.iter().filter_map(|i| i["id"].as_i64()).collect();

    // Only the answered ticket (status=open AND isanswered=true)
    assert!(ids.contains(&answered_id), "answered ticket should be in answered queue");
    assert!(!ids.contains(&open_id), "unanswered ticket should not be in answered queue");
    assert!(!ids.contains(&closed_id), "closed ticket should not be in answered queue");
}

// --- AC-3: BS-020.1 — status=assigned returns tickets assigned to me ---

#[tokio::test]
async fn status_assigned_returns_my_tickets() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping status_assigned_returns_my_tickets");
        return;
    };
    reset_pool(&pool).await;

    let prefix = unique_prefix();
    let staff_id = get_agent_staff_id(&pool).await;

    // Create test tickets: one unassigned, one assigned to agent
    let open_id = make_ticket_with_flags(&pool, &format!("{prefix}_OPEN"), "open", false, false, None).await;
    let assigned_id =
        make_ticket_with_flags(&pool, &format!("{prefix}_ASSIGNED"), "open", false, false, Some(staff_id))
            .await;

    let router = dev_app(pool.clone());
    let cookie = staff_cookie(&router).await;
    let resp = get(&router, "/api/staff/tickets?status=assigned", Some(&cookie)).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let items = body["tickets"].as_array().expect("tickets should be an array");

    let ids: Vec<i64> = items.iter().filter_map(|i| i["id"].as_i64()).collect();

    // Only the ticket assigned to the agent
    assert!(ids.contains(&assigned_id), "assigned ticket should be in assigned queue");
    assert!(!ids.contains(&open_id), "unassigned ticket should not be in assigned queue");
}

// --- AC-4: BS-020.1 — status=overdue returns open+overdue tickets ---

#[tokio::test]
async fn status_overdue_returns_overdue_tickets() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping status_overdue_returns_overdue_tickets");
        return;
    };
    reset_pool(&pool).await;

    let prefix = unique_prefix();

    // Create test tickets: one normal, one overdue
    let open_id = make_ticket_with_flags(&pool, &format!("{prefix}_OPEN"), "open", false, false, None).await;
    let overdue_id =
        make_ticket_with_flags(&pool, &format!("{prefix}_OVERDUE"), "open", false, true, None).await;

    let router = dev_app(pool.clone());
    let cookie = staff_cookie(&router).await;
    let resp = get(&router, "/api/staff/tickets?status=overdue", Some(&cookie)).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let items = body["tickets"].as_array().expect("tickets should be an array");

    let ids: Vec<i64> = items.iter().filter_map(|i| i["id"].as_i64()).collect();

    // Only the overdue ticket
    assert!(ids.contains(&overdue_id), "overdue ticket should be in overdue queue");
    assert!(!ids.contains(&open_id), "non-overdue ticket should not be in overdue queue");
}

// --- AC-5: BS-020.1 — status=closed returns closed tickets ---

#[tokio::test]
async fn status_closed_returns_closed_tickets() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping status_closed_returns_closed_tickets");
        return;
    };
    reset_pool(&pool).await;

    let prefix = unique_prefix();

    // Create test tickets: one open, one closed
    let open_id = make_ticket_with_flags(&pool, &format!("{prefix}_OPEN"), "open", false, false, None).await;
    let closed_id =
        make_ticket_with_flags(&pool, &format!("{prefix}_CLOSED"), "closed", false, false, None).await;

    let router = dev_app(pool.clone());
    let cookie = staff_cookie(&router).await;
    let resp = get(&router, "/api/staff/tickets?status=closed", Some(&cookie)).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let items = body["tickets"].as_array().expect("tickets should be an array");

    let ids: Vec<i64> = items.iter().filter_map(|i| i["id"].as_i64()).collect();

    // Only the closed ticket
    assert!(ids.contains(&closed_id), "closed ticket should be in closed queue");
    assert!(!ids.contains(&open_id), "open ticket should not be in closed queue");
}

// --- AC-6: Default status — no param defaults to open queue ---

#[tokio::test]
async fn no_status_param_defaults_to_open() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping no_status_param_defaults_to_open");
        return;
    };
    reset_pool(&pool).await;

    // Disable toggles to make the test deterministic
    sqlx::query("UPDATE config SET value = '0' WHERE key IN ('show_assigned_tickets', 'show_answered_tickets')")
        .execute(&pool)
        .await
        .unwrap();

    let prefix = unique_prefix();

    // Create one open ticket and one closed
    let open_id = make_ticket_with_flags(&pool, &format!("{prefix}_OPEN"), "open", false, false, None).await;
    let closed_id =
        make_ticket_with_flags(&pool, &format!("{prefix}_CLOSED"), "closed", false, false, None).await;

    let router = dev_app(pool.clone());
    let cookie = staff_cookie(&router).await;

    // Request WITHOUT status param
    let resp = get(&router, "/api/staff/tickets", Some(&cookie)).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let items = body["tickets"].as_array().expect("tickets should be an array");

    let ids: Vec<i64> = items.iter().filter_map(|i| i["id"].as_i64()).collect();

    // Should return open tickets (default behavior)
    assert!(ids.contains(&open_id), "open ticket should be in default queue");
    assert!(!ids.contains(&closed_id), "closed ticket should not be in default queue");
}

// --- AC-7: FS-020.11 — GET /api/staff/tickets/stats returns quick counts ---

#[tokio::test]
async fn stats_returns_quick_counts() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping stats_returns_quick_counts");
        return;
    };
    reset_pool(&pool).await;

    let prefix = unique_prefix();
    let staff_id = get_agent_staff_id(&pool).await;

    // Create varied tickets (matching the AC-7 setup):
    // 2 open, 1 answered, 1 overdue, 1 assigned, 3 closed
    make_ticket_with_flags(&pool, &format!("{prefix}_OPEN1"), "open", false, false, None).await;
    make_ticket_with_flags(&pool, &format!("{prefix}_OPEN2"), "open", false, false, None).await;
    make_ticket_with_flags(&pool, &format!("{prefix}_ANSWERED"), "open", true, false, None).await;
    make_ticket_with_flags(&pool, &format!("{prefix}_OVERDUE"), "open", false, true, None).await;
    make_ticket_with_flags(&pool, &format!("{prefix}_ASSIGNED"), "open", false, false, Some(staff_id)).await;
    make_ticket_with_flags(&pool, &format!("{prefix}_CLOSED1"), "closed", false, false, None).await;
    make_ticket_with_flags(&pool, &format!("{prefix}_CLOSED2"), "closed", false, false, None).await;
    make_ticket_with_flags(&pool, &format!("{prefix}_CLOSED3"), "closed", false, false, None).await;

    let router = dev_app(pool.clone());
    let cookie = staff_cookie(&router).await;
    let resp = get(&router, "/api/staff/tickets/stats", Some(&cookie)).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;

    // Verify stats endpoint returns expected counts.
    // After reset + seeding these specific tickets:
    // open=2 (OPEN1, OPEN2 - pure open, not answered/overdue/assigned)
    // answered=1 (ANSWERED)
    // overdue=1 (OVERDUE)
    // assigned=1 (ASSIGNED)
    // closed=3 (CLOSED1, CLOSED2, CLOSED3)
    assert_eq!(body["open"].as_i64(), Some(2), "open count mismatch");
    assert_eq!(body["answered"].as_i64(), Some(1), "answered count mismatch");
    assert_eq!(body["overdue"].as_i64(), Some(1), "overdue count mismatch");
    assert_eq!(body["assigned"].as_i64(), Some(1), "assigned count mismatch");
    assert_eq!(body["closed"].as_i64(), Some(3), "closed count mismatch");
}

// --- AC-8: Unknown status value defaults to open ---

#[tokio::test]
async fn unknown_status_defaults_to_open() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping unknown_status_defaults_to_open");
        return;
    };
    reset_pool(&pool).await;

    // Disable toggles for deterministic test
    sqlx::query("UPDATE config SET value = '0' WHERE key IN ('show_assigned_tickets', 'show_answered_tickets')")
        .execute(&pool)
        .await
        .unwrap();

    let prefix = unique_prefix();

    let open_id = make_ticket_with_flags(&pool, &format!("{prefix}_OPEN"), "open", false, false, None).await;
    let closed_id =
        make_ticket_with_flags(&pool, &format!("{prefix}_CLOSED"), "closed", false, false, None).await;

    let router = dev_app(pool.clone());
    let cookie = staff_cookie(&router).await;

    // Request with invalid status value
    let resp = get(&router, "/api/staff/tickets?status=bogus", Some(&cookie)).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let items = body["tickets"].as_array().expect("tickets should be an array");

    let ids: Vec<i64> = items.iter().filter_map(|i| i["id"].as_i64()).collect();

    // Should default to open queue, no error
    assert!(ids.contains(&open_id), "open ticket should be in queue when unknown status defaults to open");
    assert!(!ids.contains(&closed_id), "closed ticket should not be in queue when unknown status defaults to open");
}

// --- Stats endpoint requires authentication ---

#[tokio::test]
async fn stats_requires_session() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping stats_requires_session");
        return;
    };
    let router = dev_app(pool);
    let resp = get(&router, "/api/staff/tickets/stats", None).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// --- Response shape includes rightmost_column and total ---

#[tokio::test]
async fn list_response_includes_rightmost_column_and_total() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping list_response_includes_rightmost_column_and_total");
        return;
    };
    reset_pool(&pool).await;

    let prefix = unique_prefix();

    // Create two tickets
    make_ticket_with_flags(&pool, &format!("{prefix}_OPEN1"), "open", false, false, None).await;
    make_ticket_with_flags(&pool, &format!("{prefix}_OPEN2"), "open", false, false, None).await;

    let router = dev_app(pool.clone());
    let cookie = staff_cookie(&router).await;
    let resp = get(&router, "/api/staff/tickets?status=open", Some(&cookie)).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;

    // Check response shape
    assert!(body["tickets"].is_array(), "should have tickets array");
    assert!(
        body["rightmostColumn"].is_string(),
        "should have rightmostColumn"
    );
    assert!(body["pagination"]["totalCount"].is_number(), "should have totalCount");
    assert!(body["pagination"]["page"].is_number(), "should have page");
    assert!(body["pagination"]["pageSize"].is_number(), "should have pageSize");
    assert!(body["pagination"]["totalPages"].is_number(), "should have totalPages");
    // After reset, we created exactly 2 open tickets
    assert_eq!(body["pagination"]["totalCount"].as_i64(), Some(2), "total should be 2");
}
