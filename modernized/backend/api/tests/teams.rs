//! Integration tests for TS-M4-C3 — team CRUD + lead/member-removal + delete
//! releases associations.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test teams
//! ```
//!
//! @implements FS-030.10/.11/.12.
//! @implements BS-030-12/13/14/16.

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
const CSRF_HEADER: &str = "x-csrftoken";

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
    if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() }
}
async fn login(router: &Router, user: &str, pass: &str) -> (String, String) {
    let resp = router.clone().oneshot(Request::builder().method("POST").uri("/api/staff/login")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "username": user, "password": pass }).to_string())).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "login {user}");
    (set_cookie_value(&resp, "ost_staff_sess").unwrap(), set_cookie_value(&resp, "XSRF-TOKEN-STAFF").unwrap())
}
async fn get(router: &Router, uri: &str, sess: &str) -> http::Response<Body> {
    router.clone().oneshot(Request::builder().uri(uri)
        .header(COOKIE, format!("ost_staff_sess={sess}")).body(Body::empty()).unwrap()).await.unwrap()
}
async fn send(router: &Router, method: &str, uri: &str, sess: &str, csrf: &str, body: Value) -> http::Response<Body> {
    router.clone().oneshot(Request::builder().method(method).uri(uri)
        .header(COOKIE, format!("ost_staff_sess={sess}; XSRF-TOKEN-STAFF={csrf}"))
        .header(CSRF_HEADER, csrf).header("content-type", "application/json")
        .body(Body::from(body.to_string())).unwrap()).await.unwrap()
}
async fn dev(router: &Router, uri: &str, body: Value) -> http::Response<Body> {
    router.clone().oneshot(Request::builder().method("POST").uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string())).unwrap()).await.unwrap()
}
macro_rules! skip_if_no_db { ($n:expr) => {{ let Some(p)=seeded_pool().await else { eprintln!("skip {}", $n); return; }; p }}; }
fn nonce() -> u128 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() }

// --- AC-1: name validation ----------------------------------------------------

#[tokio::test]
async fn name_validation() {
    let pool = skip_if_no_db!("name_validation");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    let resp = send(&router, "POST", "/api/staff/admin/teams", &admin, &csrf, json!({ "name": "AB" })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json_body(resp).await["error"]["fields"]["name"].as_str().unwrap().contains("at least 3"));

    let resp = send(&router, "POST", "/api/staff/admin/teams", &admin, &csrf, json!({ "name": "Tier 2" })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json_body(resp).await["error"]["fields"]["name"].as_str().unwrap().to_lowercase().contains("already exists"));

    // agent → 403.
    let (agent, _) = login(&router, "agent", "Agent123!").await;
    assert_eq!(get(&router, "/api/staff/admin/teams", &agent).await.status(), StatusCode::FORBIDDEN);
}

// --- AC-2: lead must be a current member; removing the lead resets it ---------

#[tokio::test]
async fn lead_member_rules() {
    let pool = skip_if_no_db!("lead_member_rules");
    let router = dev_app(pool.clone());
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    // Create a fresh team, add `agent` as a member (via the staff-side add source), set lead.
    let team_id = json_body(send(&router, "POST", "/api/staff/admin/teams", &admin, &csrf,
        json!({ "name": format!("Squad {}", nonce()) })).await).await["id"].as_i64().unwrap();
    let agent_id: i32 = sqlx::query_scalar("SELECT staff_id FROM staff WHERE username = 'agent'").fetch_one(&pool).await.unwrap();
    let resp = send(&router, "POST", &format!("/api/staff/admin/staff/{agent_id}/teams"), &admin, &csrf, json!({ "teamId": team_id })).await;
    assert!(resp.status() == StatusCode::OK || resp.status() == StatusCode::CREATED);
    let resp = send(&router, "PUT", &format!("/api/staff/admin/teams/{team_id}"), &admin, &csrf,
        json!({ "name": format!("Squad {}", nonce()), "leadId": agent_id })).await;
    assert_eq!(resp.status(), StatusCode::OK, "lead set to a member");

    // A non-member lead → rejected.
    let resp = send(&router, "PUT", &format!("/api/staff/admin/teams/{team_id}"), &admin, &csrf,
        json!({ "name": "Squad", "leadId": 999999 })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Removing the current lead resets lead_id to NULL.
    let resp = send(&router, "PUT", &format!("/api/staff/admin/teams/{team_id}"), &admin, &csrf,
        json!({ "name": "Squad", "removeMemberIds": [agent_id] })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let team = json_body(get(&router, &format!("/api/staff/admin/teams/{team_id}"), &admin).await).await;
    assert!(team["lead_id"].is_null(), "lead reset to NULL after removal");
    assert_eq!(team["members"].as_array().unwrap().len(), 0, "member removed");
}

// --- AC-3: delete releases ticket AND help-topic associations -----------------

#[tokio::test]
async fn delete_releases_associations() {
    let pool = skip_if_no_db!("delete_releases_associations");
    let router = dev_app(pool.clone());
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    let team_id = json_body(send(&router, "POST", "/api/staff/admin/teams", &admin, &csrf,
        json!({ "name": format!("Rel {}", nonce()) })).await).await["id"].as_i64().unwrap() as i32;

    // A ticket assigned to the team + a help topic referencing it (RESTRICT FK).
    let dept_id: i32 = sqlx::query_scalar("SELECT dept_id FROM department LIMIT 1").fetch_one(&pool).await.unwrap();
    let ticket_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO ticket ("ticketID", dept_id, team_id, email, subject) VALUES ($1, $2, $3, $4, 'x') RETURNING ticket_id"#)
        .bind(710000 + (nonce() % 200000) as i64).bind(dept_id).bind(team_id).bind(format!("tm{}@ex.com", nonce()))
        .fetch_one(&pool).await.unwrap();
    let topic_id: i32 = sqlx::query_scalar(
        "INSERT INTO help_topic (topic, dept_id, team_id) VALUES ($1, $2, $3) RETURNING topic_id")
        .bind(format!("TmTopic {}", nonce())).bind(dept_id).bind(team_id).fetch_one(&pool).await.unwrap();

    let resp = send(&router, "DELETE", &format!("/api/staff/admin/teams/{team_id}"), &admin, &csrf, json!({})).await;
    assert_eq!(resp.status(), StatusCode::OK, "no FK violation");

    let t_team: i32 = sqlx::query_scalar("SELECT team_id FROM ticket WHERE ticket_id = $1").bind(ticket_id).fetch_one(&pool).await.unwrap();
    assert_eq!(t_team, 0, "ticket team_id cleared to 0");
    let h_team: Option<i32> = sqlx::query_scalar("SELECT team_id FROM help_topic WHERE topic_id = $1").bind(topic_id).fetch_one(&pool).await.unwrap();
    assert_eq!(h_team, None, "help_topic team_id set NULL");
    let members: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM team_member WHERE team_id = $1").bind(team_id).fetch_one(&pool).await.unwrap();
    assert_eq!(members, 0, "team_member rows gone");
}

// --- AC-4: the team form exposes no add-member operation ----------------------

#[tokio::test]
async fn no_add_member_field() {
    let pool = skip_if_no_db!("no_add_member_field");
    let router = dev_app(pool.clone());
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    let team_id = json_body(send(&router, "POST", "/api/staff/admin/teams", &admin, &csrf,
        json!({ "name": format!("NoAdd {}", nonce()) })).await).await["id"].as_i64().unwrap() as i32;

    // An unknown "addMemberIds" field is ignored (whitelist deserialize) — the PUT
    // still succeeds and adds nobody.
    let resp = send(&router, "PUT", &format!("/api/staff/admin/teams/{team_id}"), &admin, &csrf,
        json!({ "name": "NoAdd", "addMemberIds": [1, 2, 3] })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let members: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM team_member WHERE team_id = $1").bind(team_id).fetch_one(&pool).await.unwrap();
    assert_eq!(members, 0, "no members added via the team form");
}

// --- reset-teams restores the Tier 2 baseline --------------------------------

#[tokio::test]
async fn reset_restores_tier2() {
    let pool = skip_if_no_db!("reset_restores_tier2");
    let router = dev_app(pool);
    let (admin, _) = login(&router, "admin", "Admin123!").await;
    assert_eq!(dev(&router, "/api/dev/reset-teams", json!({})).await.status(), StatusCode::OK);
    let list = json_body(get(&router, "/api/staff/admin/teams", &admin).await).await;
    assert!(list.as_array().unwrap().iter().any(|t| t["name"] == "Tier 2"), "Tier 2 restored");
}
