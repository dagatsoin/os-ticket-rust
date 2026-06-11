//! Integration tests for the TS-M2-D2 canned-response fetch routes. Driven
//! through the router via `oneshot`; skipped (pass, log line) when
//! `TEST_DATABASE_URL` is unset.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   BLOB_ROOT=/tmp/ost-canned-test \
//!   cargo test -p api --test canned_fetch
//! ```
//!
//! @implements FS-022.14: canned list + substituted detail + §7 attachments.
//! @implements BS-022.1 / BS-022.2: enabled + dept-scoped offers; 404 disabled.

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

fn dev_app(pool: PgPool) -> Router {
    app(
        AppState::with_pool(pool).with_app_env(AppEnv::Development),
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

async fn login(router: &Router) -> String {
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/staff/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"username":"agent","password":"Agent123!"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    set_cookie_value(&resp, "ost_staff_sess").unwrap()
}

async fn make_ticket(pool: &PgPool, email: &str) -> (i64, i64) {
    let nt = NewTicket::validated(NewTicketInput {
        email: email.into(),
        name: "Canned Tester".into(),
        subject: "needs a canned reply".into(),
        body: "first message".into(),
        source: Some("Web".into()),
        dept_id: None,
    })
    .unwrap();
    let t = create_ticket(pool, &nt).await.unwrap();
    (t.ticket_id, t.ticket_number)
}

async fn get(router: &Router, uri: &str, cookie: &str) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header("cookie", format!("ost_staff_sess={cookie}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

/// The id of a seeded canned response by its title (direct DB lookup, used to
/// resolve the disabled-sample id for the 404 case).
async fn canned_id_by_title(pool: &PgPool, title: &str) -> i32 {
    sqlx::query_scalar("SELECT canned_id FROM canned_response WHERE title = $1")
        .bind(title)
        .fetch_one(pool)
        .await
        .unwrap()
}

// --- AC-1: list returns only enabled, dept-scoped responses ------------------

#[tokio::test]
async fn list_includes_enabled_excludes_disabled() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping list_includes_enabled_excludes_disabled");
        return;
    };
    let router = dev_app(pool.clone());
    let (id, _) = make_ticket(&pool, "ac1-canned@example.com").await;
    let cookie = login(&router).await;

    let resp = get(&router, &format!("/api/staff/tickets/{id}/canned"), &cookie).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    let titles: Vec<String> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["title"].as_str().unwrap().to_string())
        .collect();
    assert!(
        titles.iter().any(|t| t == "Acknowledge receipt"),
        "enabled dept-0 sample is listed: {titles:?}"
    );
    assert!(
        !titles.iter().any(|t| t == "Closed — disabled sample"),
        "disabled sample is NOT listed: {titles:?}"
    );
}

// --- AC-2 + AC-3: substituted body + §7 attachments --------------------------

#[tokio::test]
async fn detail_substitutes_body_and_lists_attachments() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping detail_substitutes_body...");
        return;
    };
    let router = dev_app(pool.clone());
    let (id, number) = make_ticket(&pool, "ac2-canned@example.com").await;
    let cookie = login(&router).await;

    // AC-2: the variable-carrying "Acknowledge receipt" body substitutes the
    // ticket number and leaves no literal %{...}.
    let ack_id = canned_id_by_title(&pool, "Acknowledge receipt").await;
    let resp = get(
        &router,
        &format!("/api/staff/tickets/{id}/canned/{ack_id}"),
        &cookie,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    let body = json["body"].as_str().unwrap();
    assert!(
        body.contains(&number.to_string()),
        "body holds the ticket number {number}: {body}"
    );
    assert!(!body.contains("%{"), "no literal %{{...}} remains: {body}");

    // AC-3: the attachment-carrying sample lists policy.txt under `attachments`
    // with the §7 shape {id, name, size, mime}.
    let policy_id = canned_id_by_title(&pool, "Sample (with attachment)").await;
    let resp = get(
        &router,
        &format!("/api/staff/tickets/{id}/canned/{policy_id}"),
        &cookie,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    let atts = json["attachments"].as_array().unwrap();
    assert_eq!(atts.len(), 1, "one canned attachment");
    let a = &atts[0];
    assert_eq!(a["name"], "policy.txt");
    assert!(a["id"].is_number(), "id present");
    assert!(a["size"].as_i64().unwrap() > 0, "size present");
    assert_eq!(a["mime"], "text/plain");
}

// --- AC-4: a disabled canned id returns 404 ----------------------------------

#[tokio::test]
async fn disabled_canned_id_returns_404() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping disabled_canned_id_returns_404");
        return;
    };
    let router = dev_app(pool.clone());
    let (id, _) = make_ticket(&pool, "ac4-canned@example.com").await;
    let cookie = login(&router).await;

    let disabled_id = canned_id_by_title(&pool, "Closed — disabled sample").await;
    let resp = get(
        &router,
        &format!("/api/staff/tickets/{id}/canned/{disabled_id}"),
        &cookie,
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "a disabled canned id is not fetchable (404)"
    );

    // An entirely unknown id is also 404.
    let resp = get(
        &router,
        &format!("/api/staff/tickets/{id}/canned/99999999"),
        &cookie,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// --- realm gate: no session ⇒ 401 -------------------------------------------

#[tokio::test]
async fn unauthenticated_list_is_401() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping unauthenticated_list_is_401");
        return;
    };
    let router = dev_app(pool.clone());
    let (id, _) = make_ticket(&pool, "ac5-canned@example.com").await;
    let resp = get(&router, &format!("/api/staff/tickets/{id}/canned"), "bogus").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
