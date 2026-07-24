//! Integration tests for TS-M3-A3: BS-020.5 listing columns per queue (rightmost column logic).
//!
//! Tests the `GET /api/staff/tickets?status={open|answered|assigned|overdue|closed}` route
//! to verify the `rightmost_column` metadata and corresponding field values. Driven through
//! the router via `oneshot`; skipped (pass, log line) when `TEST_DATABASE_URL` is unset.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test rightmost_column -- --test-threads=1
//! ```
//!
//! @implements BS-020.5: rightmost column varies by queue.

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

/// Create a ticket with specific flags set including closed_by_staff_id.
async fn make_ticket_with_flags(
    pool: &PgPool,
    subject: &str,
    status: &str,
    isanswered: bool,
    isoverdue: bool,
    staff_id: Option<i32>,
    team_id: Option<i32>,
    closed_by_staff_id: Option<i32>,
) -> i64 {
    let id = make_ticket(pool, subject).await;
    sqlx::query(
        "UPDATE ticket SET status = $1, isanswered = $2, isoverdue = $3, staff_id = $4, team_id = $5, closed_by_staff_id = $6 WHERE ticket_id = $7",
    )
    .bind(status)
    .bind(isanswered)
    .bind(isoverdue)
    .bind(staff_id)
    .bind(team_id.unwrap_or(0))
    .bind(closed_by_staff_id)
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

/// Get or create a test team, returning its team_id.
async fn get_or_create_team(pool: &PgPool, name: &str) -> i32 {
    let existing: Option<i32> = sqlx::query_scalar("SELECT team_id FROM team WHERE name = $1")
        .bind(name)
        .fetch_optional(pool)
        .await
        .unwrap();
    if let Some(id) = existing {
        return id;
    }
    sqlx::query_scalar("INSERT INTO team (name, isenabled) VALUES ($1, true) RETURNING team_id")
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap()
}

// --- AC-1: BS-020.5 — Open queue with show_assigned_tickets=1 returns rightmost_column=assigned_to ---

#[tokio::test]
async fn ac1_open_queue_with_show_assigned_on_returns_assigned_to() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping ac1_open_queue_with_show_assigned_on_returns_assigned_to");
        return;
    };
    reset_pool(&pool).await;

    // Enable show_assigned_tickets
    sqlx::query("UPDATE config SET value = '1' WHERE key = 'show_assigned_tickets'")
        .execute(&pool)
        .await
        .unwrap();

    let prefix = unique_prefix();
    let staff_id = get_agent_staff_id(&pool).await;

    // Create an open ticket assigned to agent
    make_ticket_with_flags(
        &pool,
        &format!("{prefix}_OPEN_ASSIGNED"),
        "open",
        false,
        false,
        Some(staff_id),
        None,
        None,
    )
    .await;

    let router = dev_app(pool.clone());
    let cookie = staff_cookie(&router).await;
    let resp = get(&router, "/api/staff/tickets?status=open", Some(&cookie)).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;

    // Should have rightmostColumn = "assigned_to"
    assert_eq!(
        body["rightmostColumn"].as_str(),
        Some("assigned_to"),
        "Open queue with show_assigned_tickets=1 should return rightmost_column=assigned_to"
    );

    // Each item should have assignedToName field
    let items = body["tickets"].as_array().expect("tickets should be an array");
    assert!(!items.is_empty(), "should have at least one ticket");
    for item in items {
        assert!(
            item.get("assignedToName").is_some(),
            "each item should have assignedToName field when rightmost_column=assigned_to"
        );
    }
}

// --- AC-2: BS-020.5 — Open queue with show_assigned_tickets=0 returns rightmost_column=department ---

#[tokio::test]
async fn ac2_open_queue_with_show_assigned_off_returns_department() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping ac2_open_queue_with_show_assigned_off_returns_department");
        return;
    };
    reset_pool(&pool).await;

    // Disable show_assigned_tickets
    sqlx::query("UPDATE config SET value = '0' WHERE key = 'show_assigned_tickets'")
        .execute(&pool)
        .await
        .unwrap();

    let prefix = unique_prefix();

    // Create an open ticket (unassigned, since show_assigned=0)
    make_ticket_with_flags(
        &pool,
        &format!("{prefix}_OPEN"),
        "open",
        false,
        false,
        None,
        None,
        None,
    )
    .await;

    let router = dev_app(pool.clone());
    let cookie = staff_cookie(&router).await;
    let resp = get(&router, "/api/staff/tickets?status=open", Some(&cookie)).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;

    // Should have rightmostColumn = "department"
    assert_eq!(
        body["rightmostColumn"].as_str(),
        Some("department"),
        "Open queue with show_assigned_tickets=0 should return rightmost_column=department"
    );

    // Each item should have departmentName field
    let items = body["tickets"].as_array().expect("tickets should be an array");
    assert!(!items.is_empty(), "should have at least one ticket");
    for item in items {
        assert!(
            item.get("departmentName").is_some(),
            "each item should have departmentName field when rightmost_column=department"
        );
    }
}

// --- AC-3: BS-020.5 — Closed queue returns rightmost_column=closed_by ---

#[tokio::test]
async fn ac3_closed_queue_returns_closed_by() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping ac3_closed_queue_returns_closed_by");
        return;
    };
    reset_pool(&pool).await;

    let prefix = unique_prefix();
    let staff_id = get_agent_staff_id(&pool).await;

    // Create a closed ticket with closed_by_staff_id set
    make_ticket_with_flags(
        &pool,
        &format!("{prefix}_CLOSED"),
        "closed",
        false,
        false,
        None,
        None,
        Some(staff_id),
    )
    .await;

    let router = dev_app(pool.clone());
    let cookie = staff_cookie(&router).await;
    let resp = get(&router, "/api/staff/tickets?status=closed", Some(&cookie)).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;

    // Should have rightmostColumn = "closed_by"
    assert_eq!(
        body["rightmostColumn"].as_str(),
        Some("closed_by"),
        "Closed queue should return rightmost_column=closed_by"
    );

    // Each item should have closedByName field
    let items = body["tickets"].as_array().expect("tickets should be an array");
    assert!(!items.is_empty(), "should have at least one ticket");
    for item in items {
        assert!(
            item.get("closedByName").is_some(),
            "each item should have closedByName field when rightmost_column=closed_by"
        );
        // The closedByName should be "agent" (or the agent's display name)
        let name = item["closedByName"].as_str().unwrap_or("");
        assert!(!name.is_empty(), "closedByName should not be empty when closed_by_staff_id is set");
    }
}

// --- AC-4: BS-020.5 — Answered queue returns rightmost_column=assigned_to ---

#[tokio::test]
async fn ac4_answered_queue_returns_assigned_to() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping ac4_answered_queue_returns_assigned_to");
        return;
    };
    reset_pool(&pool).await;

    let prefix = unique_prefix();

    // Create an answered ticket
    make_ticket_with_flags(
        &pool,
        &format!("{prefix}_ANSWERED"),
        "open",
        true,
        false,
        None,
        None,
        None,
    )
    .await;

    let router = dev_app(pool.clone());
    let cookie = staff_cookie(&router).await;
    let resp = get(&router, "/api/staff/tickets?status=answered", Some(&cookie)).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;

    // Should have rightmostColumn = "assigned_to"
    assert_eq!(
        body["rightmostColumn"].as_str(),
        Some("assigned_to"),
        "Answered queue should return rightmost_column=assigned_to"
    );
}

// --- AC-5: BS-020.5 — Overdue queue returns rightmost_column=assigned_to ---

#[tokio::test]
async fn ac5_overdue_queue_returns_assigned_to() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping ac5_overdue_queue_returns_assigned_to");
        return;
    };
    reset_pool(&pool).await;

    let prefix = unique_prefix();

    // Create an overdue ticket
    make_ticket_with_flags(
        &pool,
        &format!("{prefix}_OVERDUE"),
        "open",
        false,
        true,
        None,
        None,
        None,
    )
    .await;

    let router = dev_app(pool.clone());
    let cookie = staff_cookie(&router).await;
    let resp = get(&router, "/api/staff/tickets?status=overdue", Some(&cookie)).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;

    // Should have rightmostColumn = "assigned_to"
    assert_eq!(
        body["rightmostColumn"].as_str(),
        Some("assigned_to"),
        "Overdue queue should return rightmost_column=assigned_to"
    );
}

// --- AC-6: BS-020.5 — Assigned (My Tickets) queue returns rightmost_column=department ---

#[tokio::test]
async fn ac6_assigned_queue_returns_department() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping ac6_assigned_queue_returns_department");
        return;
    };
    reset_pool(&pool).await;

    let prefix = unique_prefix();
    let staff_id = get_agent_staff_id(&pool).await;

    // Create a ticket assigned to the agent
    make_ticket_with_flags(
        &pool,
        &format!("{prefix}_ASSIGNED"),
        "open",
        false,
        false,
        Some(staff_id),
        None,
        None,
    )
    .await;

    let router = dev_app(pool.clone());
    let cookie = staff_cookie(&router).await;
    let resp = get(&router, "/api/staff/tickets?status=assigned", Some(&cookie)).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;

    // Should have rightmostColumn = "department" (Assigned To is redundant when it's always "me")
    assert_eq!(
        body["rightmostColumn"].as_str(),
        Some("department"),
        "Assigned queue should return rightmost_column=department (assignee is always 'me')"
    );

    // Each item should have departmentName field
    let items = body["tickets"].as_array().expect("tickets should be an array");
    assert!(!items.is_empty(), "should have at least one ticket");
    for item in items {
        assert!(
            item.get("departmentName").is_some(),
            "each item should have departmentName field when rightmost_column=department"
        );
    }
}

// --- AC-7: Assigned-to shows staff name when assigned to staff ---

#[tokio::test]
async fn ac7_assigned_to_shows_staff_name() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping ac7_assigned_to_shows_staff_name");
        return;
    };
    reset_pool(&pool).await;

    // Enable show_assigned_tickets so assigned_to is visible
    sqlx::query("UPDATE config SET value = '1' WHERE key = 'show_assigned_tickets'")
        .execute(&pool)
        .await
        .unwrap();

    let prefix = unique_prefix();
    let staff_id = get_agent_staff_id(&pool).await;

    // Create an open ticket assigned to agent (staff_id set, no team_id)
    let ticket_id = make_ticket_with_flags(
        &pool,
        &format!("{prefix}_STAFF_ASSIGNED"),
        "open",
        false,
        false,
        Some(staff_id),
        None, // no team
        None,
    )
    .await;

    let router = dev_app(pool.clone());
    let cookie = staff_cookie(&router).await;
    let resp = get(&router, "/api/staff/tickets?status=open", Some(&cookie)).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let items = body["tickets"].as_array().expect("tickets should be an array");

    // Find our ticket
    let item = items
        .iter()
        .find(|i| i["id"].as_i64() == Some(ticket_id))
        .expect("should find the created ticket");

    // assignedToName should be the staff name (agent's display name)
    let name = item["assignedToName"].as_str().unwrap_or("");
    assert!(
        !name.is_empty(),
        "assignedToName should not be empty when staff_id is set"
    );
    // The seeded agent has username "agent", so name should include "agent" or be "agent"
    // (depending on whether firstname/lastname are set)
}

// --- AC-8: Assigned-to shows team name when assigned to team only ---

#[tokio::test]
async fn ac8_assigned_to_shows_team_name() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping ac8_assigned_to_shows_team_name");
        return;
    };
    reset_pool(&pool).await;

    // Enable show_assigned_tickets so assigned_to is visible
    sqlx::query("UPDATE config SET value = '1' WHERE key = 'show_assigned_tickets'")
        .execute(&pool)
        .await
        .unwrap();

    let prefix = unique_prefix();
    let team_id = get_or_create_team(&pool, "Tier 2").await;

    // Create an open ticket assigned to team only (no staff_id)
    let ticket_id = make_ticket_with_flags(
        &pool,
        &format!("{prefix}_TEAM_ASSIGNED"),
        "open",
        false,
        false,
        None, // no staff
        Some(team_id),
        None,
    )
    .await;

    let router = dev_app(pool.clone());
    let cookie = staff_cookie(&router).await;
    let resp = get(&router, "/api/staff/tickets?status=open", Some(&cookie)).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let items = body["tickets"].as_array().expect("tickets should be an array");

    // Find our ticket
    let item = items
        .iter()
        .find(|i| i["id"].as_i64() == Some(ticket_id))
        .expect("should find the created ticket");

    // assignedToName should be the team name "Tier 2"
    let name = item["assignedToName"].as_str().unwrap_or("");
    assert_eq!(
        name, "Tier 2",
        "assignedToName should be the team name when only team_id is set"
    );
}
