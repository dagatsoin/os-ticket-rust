//! Integration test for `GET /api/staff/ticket-options` — the agent-facing
//! transfer/assign/edit reference lists.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test ticket_options
//! ```
//!
//! @implements FS-032.10/.11/.12: staff transfer/assign/edit reference data.
//! @implements BS-020.2: read-only reference lists for any staff realm session.

use api::{app, AppEnv, AppState};
use axum::body::Body;
use axum::Router;
use http::header::{COOKIE, SET_COOKIE};
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
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
fn set_cookie_value(resp: &http::Response<Body>, name: &str) -> Option<String> {
    for hv in resp.headers().get_all(SET_COOKIE) {
        let s = hv.to_str().ok()?;
        let first = s.split(';').next().unwrap_or("");
        if let Some(v) = first.strip_prefix(&format!("{name}=")) {
            return Some(v.to_string());
        }
    }
    None
}
async fn json_body(resp: http::Response<Body>) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}
async fn login(router: &Router, user: &str, pass: &str) -> String {
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/staff/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "username": user, "password": pass }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "login {user}");
    set_cookie_value(&resp, "ost_staff_sess").unwrap()
}
async fn get(router: &Router, uri: &str, sess: Option<&str>) -> http::Response<Body> {
    let mut b = Request::builder().uri(uri);
    if let Some(s) = sess {
        b = b.header(COOKIE, format!("ost_staff_sess={s}"));
    }
    router
        .clone()
        .oneshot(b.body(Body::empty()).unwrap())
        .await
        .unwrap()
}
macro_rules! skip_if_no_db {
    ($n:expr) => {{
        let Some(p) = seeded_pool().await else {
            eprintln!("skip {}", $n);
            return;
        };
        p
    }};
}

/// Collect the `name` field of every `{id,name}` object in an array.
fn names(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|o| o["name"].as_str().unwrap().to_string())
        .collect()
}

/// A plain agent (NOT admin) gets 200 with the 6 populated reference arrays.
#[tokio::test]
async fn agent_gets_populated_reference_lists() {
    let pool = skip_if_no_db!("agent_gets_populated_reference_lists");
    let router = dev_app(pool);
    let agent = login(&router, "agent", "Agent123!").await;

    let resp = get(&router, "/api/staff/ticket-options", Some(&agent)).await;
    assert_eq!(resp.status(), StatusCode::OK, "agent (non-admin) is allowed");
    let body = json_body(resp).await;

    // All six keys present and shaped as `[{id,name}]`.
    for key in [
        "departments",
        "agents",
        "teams",
        "help_topics",
        "priorities",
        "sla_plans",
    ] {
        let arr = body[key].as_array().unwrap_or_else(|| panic!("{key} is an array"));
        assert!(!arr.is_empty(), "{key} must be populated from the seed");
        for item in arr {
            assert!(item["id"].is_i64() || item["id"].is_u64(), "{key} id is int");
            assert!(item["name"].is_string(), "{key} name is string");
        }
    }

    // Seed expectations.
    let depts = names(&body["departments"]);
    assert!(depts.contains(&"Support".to_string()), "Support dept present");
    assert!(depts.contains(&"Sales".to_string()), "Sales dept present");

    // agents = active staff by display name (firstname+lastname); the seeded
    // `agent`/`agent2` accounts render as "Agent One"/"Agent Two".
    let agents = names(&body["agents"]);
    assert!(agents.contains(&"Agent One".to_string()), "agent in agents: {agents:?}");
    assert!(agents.contains(&"Agent Two".to_string()), "agent2 in agents: {agents:?}");

    let teams = names(&body["teams"]);
    assert!(teams.contains(&"Tier 2".to_string()), "Tier 2 team enabled");

    let topics = names(&body["help_topics"]);
    assert!(topics.contains(&"General".to_string()), "General topic active");
    assert!(topics.contains(&"Billing".to_string()), "Billing topic active");

    // priorities: 4 of them, ordered by urgency DESC (Low, Normal, High, Emergency
    // have urgency 4,3,2,1 respectively — DESC yields Low..Emergency).
    let priorities = names(&body["priorities"]);
    assert_eq!(priorities.len(), 4, "four seeded priorities");
    assert_eq!(
        priorities,
        vec!["Low", "Normal", "High", "Emergency"],
        "priorities ordered by urgency DESC"
    );

    assert!(
        !body["sla_plans"].as_array().unwrap().is_empty(),
        "at least one active SLA plan seeded"
    );
}

/// No session → 401 (StaffSession gate).
#[tokio::test]
async fn no_session_is_unauthorized() {
    let pool = skip_if_no_db!("no_session_is_unauthorized");
    let router = dev_app(pool);

    let resp = get(&router, "/api/staff/ticket-options", None).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
