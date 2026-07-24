//! Integration tests for TS-M4-C5 — help-topic CRUD + routing + one-level nesting
//! + delete-promotes-children.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test help_topics
//! ```
//!
//! @implements FS-030.15/.16/.17.
//! @implements BS-030-18/19/20/22/23/26.

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

async fn a_dept(router: &Router, sess: &str) -> i64 {
    let opts = json_body(get(router, "/api/staff/admin/help-topics/form-options", sess).await).await;
    opts["departments"][0]["id"].as_i64().expect("a department")
}

// --- AC-1: validation ---------------------------------------------------------

#[tokio::test]
async fn topic_validation() {
    let pool = skip_if_no_db!("topic_validation");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let dept = a_dept(&router, &admin).await;

    // 4-char topic → "5 chars minimum".
    let resp = send(&router, "POST", "/api/staff/admin/help-topics", &admin, &csrf,
        json!({ "topic": "Abcd", "deptId": dept, "priorityId": 2 })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json_body(resp).await["error"]["fields"]["topic"], "5 chars minimum");

    // Valid topic, no dept.
    let resp = send(&router, "POST", "/api/staff/admin/help-topics", &admin, &csrf,
        json!({ "topic": format!("Topic {}", nonce()), "priorityId": 2 })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json_body(resp).await["error"]["fields"]["deptId"], "You must select a department");

    // Valid topic, dept but no priority.
    let resp = send(&router, "POST", "/api/staff/admin/help-topics", &admin, &csrf,
        json!({ "topic": format!("Topic {}", nonce()), "deptId": dept })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json_body(resp).await["error"]["fields"]["priorityId"], "You must select a priority");

    // agent → 403.
    let (agent, _) = login(&router, "agent", "Agent123!").await;
    assert_eq!(get(&router, "/api/staff/admin/help-topics", &agent).await.status(), StatusCode::FORBIDDEN);
}

// --- AC-2: parent options top-level only AND write-side one-level enforcement --

#[tokio::test]
async fn one_level_nesting() {
    let pool = skip_if_no_db!("one_level_nesting");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let dept = a_dept(&router, &admin).await;

    // Seed a parent + a child (parentId = parent).
    let parent = json_body(send(&router, "POST", "/api/staff/admin/help-topics", &admin, &csrf,
        json!({ "topic": format!("Parent {}", nonce()), "deptId": dept, "priorityId": 2 })).await).await["id"].as_i64().unwrap();
    let child = json_body(send(&router, "POST", "/api/staff/admin/help-topics", &admin, &csrf,
        json!({ "topic": format!("Child {}", nonce()), "deptId": dept, "priorityId": 2, "parentId": parent })).await).await["id"].as_i64().unwrap();

    // form-options parent_topics excludes the child (topic_pid != 0).
    let opts = json_body(get(&router, "/api/staff/admin/help-topics/form-options", &admin).await).await;
    let parents: Vec<i64> = opts["parent_topics"].as_array().unwrap().iter().map(|p| p["id"].as_i64().unwrap()).collect();
    assert!(parents.contains(&parent), "top-level topic offered as parent");
    assert!(!parents.contains(&child), "child topic NOT offered as parent");

    // Write-side: parentId pointing at the child → 422.
    let resp = send(&router, "POST", "/api/staff/admin/help-topics", &admin, &csrf,
        json!({ "topic": format!("Grandchild {}", nonce()), "deptId": dept, "priorityId": 2, "parentId": child })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY, "one-level nesting enforced write-side");
}

// --- WARN-4b: a parent-with-children may not itself be nested (no 3-level chain)

#[tokio::test]
async fn cannot_nest_a_topic_that_already_has_children() {
    let pool = skip_if_no_db!("cannot_nest_a_topic_that_already_has_children");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let dept = a_dept(&router, &admin).await;

    // A top-level parent with one child (parent now HAS children).
    let parent = json_body(send(&router, "POST", "/api/staff/admin/help-topics", &admin, &csrf,
        json!({ "topic": format!("MidParent {}", nonce()), "deptId": dept, "priorityId": 2 })).await).await["id"].as_i64().unwrap();
    let _child = json_body(send(&router, "POST", "/api/staff/admin/help-topics", &admin, &csrf,
        json!({ "topic": format!("MidChild {}", nonce()), "deptId": dept, "priorityId": 2, "parentId": parent })).await).await["id"].as_i64().unwrap();

    // A separate top-level topic to try to nest the parent under.
    let grandparent = json_body(send(&router, "POST", "/api/staff/admin/help-topics", &admin, &csrf,
        json!({ "topic": format!("GrandTop {}", nonce()), "deptId": dept, "priorityId": 2 })).await).await["id"].as_i64().unwrap();

    // PUT the parent giving it a parent → would form child → parent → grandparent
    // (3 levels). Rejected 422 on parentId (WARN-4b / KL-030-02).
    let resp = send(&router, "PUT", &format!("/api/staff/admin/help-topics/{parent}"), &admin, &csrf,
        json!({ "topic": "MidParent edit", "deptId": dept, "priorityId": 2, "parentId": grandparent })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY, "nesting a parent-with-children is rejected");
    assert!(json_body(resp).await["error"]["fields"]["parentId"].is_string());
}

// --- AC-3: auto-assign is staff XOR team (`s`/`t` encoding) --------------------

#[tokio::test]
async fn assign_to_encoding() {
    let pool = skip_if_no_db!("assign_to_encoding");
    let router = dev_app(pool.clone());
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let dept = a_dept(&router, &admin).await;
    let staff_id: i32 = sqlx::query_scalar("SELECT staff_id FROM staff WHERE username = 'agent'").fetch_one(&pool).await.unwrap();
    let team_id: i32 = sqlx::query_scalar("SELECT team_id FROM team WHERE name = 'Tier 2'").fetch_one(&pool).await.unwrap();

    // Encode a staff assignee.
    let topic = json_body(send(&router, "POST", "/api/staff/admin/help-topics", &admin, &csrf,
        json!({ "topic": format!("Assign {}", nonce()), "deptId": dept, "priorityId": 2, "assignTo": format!("s{staff_id}") })).await).await["id"].as_i64().unwrap();
    let t = json_body(get(&router, &format!("/api/staff/admin/help-topics/{topic}"), &admin).await).await;
    assert_eq!(t["staff_id"], staff_id, "staff assignee retained");
    assert!(t["team_id"].is_null(), "team NULL");

    // PUT a team assignee → staff cleared.
    let resp = send(&router, "PUT", &format!("/api/staff/admin/help-topics/{topic}"), &admin, &csrf,
        json!({ "topic": "Assign", "deptId": dept, "priorityId": 2, "assignTo": format!("t{team_id}") })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let t = json_body(get(&router, &format!("/api/staff/admin/help-topics/{topic}"), &admin).await).await;
    assert_eq!(t["team_id"], team_id, "team assignee retained");
    assert!(t["staff_id"].is_null(), "staff cleared");

    // Neither → both NULL.
    let resp = send(&router, "PUT", &format!("/api/staff/admin/help-topics/{topic}"), &admin, &csrf,
        json!({ "topic": "Assign", "deptId": dept, "priorityId": 2 })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let t = json_body(get(&router, &format!("/api/staff/admin/help-topics/{topic}"), &admin).await).await;
    assert!(t["staff_id"].is_null() && t["team_id"].is_null(), "both NULL when neither supplied");
}

// --- AC-4: thank-you page + SLA override optional -----------------------------

#[tokio::test]
async fn thank_you_page_and_sla_optional() {
    let pool = skip_if_no_db!("thank_you_page_and_sla_optional");
    let router = dev_app(pool);
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let dept = a_dept(&router, &admin).await;

    let page_id = json_body(dev(&router, "/api/dev/seed-page", json!({ "name": format!("TY {}", nonce()), "type": "thank-you" })).await).await["id"].as_i64().unwrap();

    let topic = json_body(send(&router, "POST", "/api/staff/admin/help-topics", &admin, &csrf,
        json!({ "topic": format!("TYTopic {}", nonce()), "deptId": dept, "priorityId": 2, "pageId": page_id, "slaId": 0 })).await).await["id"].as_i64().unwrap();
    let t = json_body(get(&router, &format!("/api/staff/admin/help-topics/{topic}"), &admin).await).await;
    assert_eq!(t["page_id"], page_id, "thank-you page bound");
    assert!(t["sla_id"].is_null(), "SLA 0 persists as no topic SLA");

    // form-options pages are thank-you only.
    let opts = json_body(get(&router, "/api/staff/admin/help-topics/form-options", &admin).await).await;
    assert!(opts["pages"].as_array().unwrap().iter().any(|p| p["id"].as_i64() == Some(page_id)), "thank-you page offered");
}

// --- AC-5: delete promotes children + clears references -----------------------

#[tokio::test]
async fn delete_promotes_children() {
    let pool = skip_if_no_db!("delete_promotes_children");
    let router = dev_app(pool.clone());
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let dept = a_dept(&router, &admin).await;

    let parent = json_body(send(&router, "POST", "/api/staff/admin/help-topics", &admin, &csrf,
        json!({ "topic": format!("DelParent {}", nonce()), "deptId": dept, "priorityId": 2 })).await).await["id"].as_i64().unwrap();
    let child = json_body(send(&router, "POST", "/api/staff/admin/help-topics", &admin, &csrf,
        json!({ "topic": format!("DelChild {}", nonce()), "deptId": dept, "priorityId": 2, "parentId": parent })).await).await["id"].as_i64().unwrap() as i32;

    // A ticket referencing the parent topic.
    let dept_id: i32 = sqlx::query_scalar("SELECT dept_id FROM department LIMIT 1").fetch_one(&pool).await.unwrap();
    let ticket_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO ticket ("ticketID", dept_id, topic_id, email, subject) VALUES ($1, $2, $3, $4, 'x') RETURNING ticket_id"#)
        .bind(720000 + (nonce() % 200000) as i64).bind(dept_id).bind(parent as i32).bind(format!("tp{}@ex.com", nonce()))
        .fetch_one(&pool).await.unwrap();

    let resp = send(&router, "DELETE", &format!("/api/staff/admin/help-topics/{parent}"), &admin, &csrf, json!({})).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let child_pid: i32 = sqlx::query_scalar("SELECT topic_pid FROM help_topic WHERE topic_id = $1").bind(child).fetch_one(&pool).await.unwrap();
    assert_eq!(child_pid, 0, "child promoted to top-level");
    let t_topic: i32 = sqlx::query_scalar("SELECT topic_id FROM ticket WHERE ticket_id = $1").bind(ticket_id).fetch_one(&pool).await.unwrap();
    assert_eq!(t_topic, 0, "ticket topic_id cleared to 0");
}

// --- list is paginated --------------------------------------------------------

#[tokio::test]
async fn list_is_paginated() {
    let pool = skip_if_no_db!("list_is_paginated");
    let router = dev_app(pool);
    let (admin, _) = login(&router, "admin", "Admin123!").await;
    let body = json_body(get(&router, "/api/staff/admin/help-topics", &admin).await).await;
    assert!(body["items"].is_array());
    assert_eq!(body["page_size"], 25);
    assert!(body["total"].as_i64().is_some());
}
