//! Integration tests for TS-M4-G1 (write-side) + TS-M4-G2 (viewer/purge).
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test logs
//! ```
//!
//! @implements FS-033.1–.8 + BS-033.6 (DEVIATION M4-D2).

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

fn test_db_url() -> Option<String> { std::env::var("TEST_DATABASE_URL").ok() }
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
        if let Some(v) = first.strip_prefix(&format!("{name}=")) { return Some(v.to_string()); }
    }
    None
}
async fn json_body(resp: http::Response<Body>) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() }
}
async fn login(router: &Router, user: &str, pass: &str) -> (String, String) {
    let resp = login_raw(router, user, pass).await;
    assert_eq!(resp.status(), StatusCode::OK, "login {user}");
    (set_cookie_value(&resp, "ost_staff_sess").unwrap(), set_cookie_value(&resp, "XSRF-TOKEN-STAFF").unwrap())
}
async fn login_raw(router: &Router, user: &str, pass: &str) -> http::Response<Body> {
    router.clone().oneshot(Request::builder().method("POST").uri("/api/staff/login")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "username": user, "password": pass }).to_string())).unwrap())
        .await.unwrap()
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
async fn clear_syslog(pool: &PgPool) { sqlx::query("DELETE FROM syslog").execute(pool).await.unwrap(); }
async fn set_cfg(router: &Router, key: &str, val: &str) { dev(router, "/api/dev/seed-config", json!({ key: val })).await; }
async fn syslog_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM syslog").fetch_one(pool).await.unwrap()
}

// --- G1 AC-1: log() writes a syslog row on admin CRUD ------------------------

#[tokio::test]
async fn admin_crud_writes_syslog_row() {
    let pool = skip_if_no_db!("admin_crud_writes_syslog_row");
    let router = dev_app(pool.clone());
    set_cfg(&router, "log_level", "3").await; // Debug threshold
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    clear_syslog(&pool).await;

    send(&router, "POST", "/api/staff/admin/groups", &admin, &csrf,
        json!({ "name": format!("LogGrp {}", nonce()), "enabled": true, "flags": {}, "deptIds": [] })).await;

    let created: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM syslog WHERE title = 'Group created'")
        .fetch_one(&pool).await.unwrap();
    assert!(created >= 1, "a 'Group created' syslog row was written");
}

// --- G1 AC-2: log_level threshold suppresses below-threshold entries ---------

#[tokio::test]
async fn threshold_suppresses_below_level() {
    let pool = skip_if_no_db!("threshold_suppresses_below_level");
    let router = dev_app(pool.clone());
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;

    // Threshold = Error(1): a Debug create must NOT persist.
    set_cfg(&router, "log_level", "Error").await;
    clear_syslog(&pool).await;
    send(&router, "POST", "/api/staff/admin/groups", &admin, &csrf,
        json!({ "name": format!("Sup {}", nonce()), "enabled": true, "flags": {}, "deptIds": [] })).await;
    let debug_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM syslog WHERE log_type = 'Debug'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(debug_rows, 0, "Debug entries suppressed at log_level=Error");

    // Direct writer check: at threshold Error, an Error persists but Debug does not.
    clear_syslog(&pool).await;
    ost_core::log(&pool, ost_core::LogType::Error, "T-err", "x", "").await;
    ost_core::log(&pool, ost_core::LogType::Debug, "T-dbg", "x", "").await;
    let errs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM syslog WHERE title = 'T-err'").fetch_one(&pool).await.unwrap();
    let dbgs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM syslog WHERE title = 'T-dbg'").fetch_one(&pool).await.unwrap();
    assert_eq!(errs, 1, "Error persists at threshold Error");
    assert_eq!(dbgs, 0, "Debug suppressed at threshold Error");
}

// --- G1 AC-3: a failing log write does not fail the operation ----------------

#[tokio::test]
async fn log_write_is_best_effort() {
    let pool = skip_if_no_db!("log_write_is_best_effort");
    // A closed pool makes every query error; log() must still return () (no panic,
    // no propagation) — proving the writer is best-effort.
    let dead = db::connect(&test_db_url().unwrap()).await.unwrap();
    dead.close().await;
    // Completes without panicking or propagating.
    ost_core::log(&dead, ost_core::LogType::Debug, "won't-write", "detail", "1.2.3.4").await;
    ost_core::log(&dead, ost_core::LogType::Error, "won't-write", "detail", "1.2.3.4").await;

    // And the observed admin op still succeeds end-to-end (AC-1 covers the write).
    let router = dev_app(pool.clone());
    set_cfg(&router, "log_level", "3").await;
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let resp = send(&router, "POST", "/api/staff/admin/groups", &admin, &csrf,
        json!({ "name": format!("Beff {}", nonce()), "enabled": true, "flags": {}, "deptIds": [] })).await;
    assert_eq!(resp.status(), StatusCode::OK, "operation succeeds regardless of logging");
}

// --- G1 AC-4: seed-log inserts a backdated row -------------------------------

#[tokio::test]
async fn seed_log_backdated_row() {
    let pool = skip_if_no_db!("seed_log_backdated_row");
    let router = dev_app(pool.clone());
    let resp = dev(&router, "/api/dev/seed-log",
        json!({ "type": "Debug", "title": "old-row", "created": "2020-01-01T00:00:00Z" })).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let id = json_body(resp).await["id"].as_i64().unwrap();
    let year: Option<f64> = sqlx::query_scalar("SELECT EXTRACT(YEAR FROM created)::float8 FROM syslog WHERE id = $1")
        .bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(year, Some(2020.0), "row stamped with the backdated created");
}

// --- G1 AC-5: login success AND failure both emit rows -----------------------

#[tokio::test]
async fn login_success_and_failure_logged() {
    let pool = skip_if_no_db!("login_success_and_failure_logged");
    let router = dev_app(pool.clone());
    set_cfg(&router, "log_level", "3").await;
    clear_syslog(&pool).await;

    let _ = login_raw(&router, "admin", "WRONGpass").await; // failure → Warning
    let _ = login_raw(&router, "admin", "Admin123!").await; // success → Debug

    let failed: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM syslog WHERE title = 'Staff login failed'").fetch_one(&pool).await.unwrap();
    let ok: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM syslog WHERE title = 'Staff login'").fetch_one(&pool).await.unwrap();
    assert!(failed >= 1, "failed login logged");
    assert!(ok >= 1, "successful login logged");
}

// --- G2 AC-1: filter by type + date span -------------------------------------

#[tokio::test]
async fn filter_by_type_and_date_span() {
    let pool = skip_if_no_db!("filter_by_type_and_date_span");
    let router = dev_app(pool.clone());
    let (admin, _) = login(&router, "admin", "Admin123!").await;
    clear_syslog(&pool).await;

    dev(&router, "/api/dev/seed-log", json!({ "type": "Error", "title": "e-2021", "created": "2021-06-01T00:00:00Z" })).await;
    dev(&router, "/api/dev/seed-log", json!({ "type": "Error", "title": "e-2019", "created": "2019-06-01T00:00:00Z" })).await;
    dev(&router, "/api/dev/seed-log", json!({ "type": "Warning", "title": "w-2021", "created": "2021-06-01T00:00:00Z" })).await;

    let resp = get(&router, "/api/staff/admin/logs?type=Error&from=2021-01-01&to=2021-12-31", &admin).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let titles: Vec<&str> = body["items"].as_array().unwrap().iter().map(|r| r["title"].as_str().unwrap()).collect();
    assert_eq!(titles, vec!["e-2021"], "only Error rows within the span");
    // The list omits the big body field.
    assert!(body["items"][0].get("log").is_none(), "list omits the big log body");
}

// --- G2 AC-2: sort + paginate (newest-first default) -------------------------

#[tokio::test]
async fn sort_and_paginate() {
    let pool = skip_if_no_db!("sort_and_paginate");
    let router = dev_app(pool.clone());
    let (admin, _) = login(&router, "admin", "Admin123!").await;
    clear_syslog(&pool).await;
    for i in 0..5 {
        dev(&router, "/api/dev/seed-log", json!({ "type": "Debug", "title": format!("row{i}") })).await;
    }
    let p1 = json_body(get(&router, "/api/staff/admin/logs?per_page=2&page=1", &admin).await).await;
    let p2 = json_body(get(&router, "/api/staff/admin/logs?per_page=2&page=2", &admin).await).await;
    assert_eq!(p1["per_page"], 2);
    assert!(p1["total"].as_i64().unwrap() >= 5);
    let id1 = p1["items"][0]["id"].as_i64().unwrap();
    let id2 = p2["items"][0]["id"].as_i64().unwrap();
    assert_ne!(id1, id2, "page 2 differs from page 1");
    // Default order is newest-first (highest id first).
    assert!(p1["items"][0]["id"].as_i64().unwrap() > p1["items"][1]["id"].as_i64().unwrap(), "newest-first default");
}

// --- G2 AC-3: single record detail -------------------------------------------

#[tokio::test]
async fn single_record_detail() {
    let pool = skip_if_no_db!("single_record_detail");
    let router = dev_app(pool.clone());
    let (admin, _) = login(&router, "admin", "Admin123!").await;
    let id = json_body(dev(&router, "/api/dev/seed-log",
        json!({ "type": "Warning", "title": "detail-me", "log": "the full body text" })).await).await["id"].as_i64().unwrap();

    let resp = get(&router, &format!("/api/staff/admin/logs/{id}"), &admin).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["title"], "detail-me");
    assert_eq!(body["log"], "the full body text");

    assert_eq!(get(&router, "/api/staff/admin/logs/99999999", &admin).await.status(), StatusCode::NOT_FOUND);
}

// --- G2 AC-4: bulk deletion --------------------------------------------------

#[tokio::test]
async fn bulk_delete() {
    let pool = skip_if_no_db!("bulk_delete");
    let router = dev_app(pool.clone());
    let (admin, csrf) = login(&router, "admin", "Admin123!").await;
    let a = json_body(dev(&router, "/api/dev/seed-log", json!({ "type": "Debug", "title": "del-a" })).await).await["id"].as_i64().unwrap();
    let b = json_body(dev(&router, "/api/dev/seed-log", json!({ "type": "Debug", "title": "del-b" })).await).await["id"].as_i64().unwrap();

    let resp = send(&router, "POST", "/api/staff/admin/logs/delete", &admin, &csrf, json!({ "ids": [a, b] })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["affected"], 2);
    let gone: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM syslog WHERE id = ANY($1)").bind(vec![a, b])
        .fetch_one(&pool).await.unwrap();
    assert_eq!(gone, 0, "deleted rows no longer present");
}

// --- G2 AC-5: purge sweep deletes over-age rows only -------------------------

#[tokio::test]
async fn purge_sweep_over_age_only() {
    let pool = skip_if_no_db!("purge_sweep_over_age_only");
    let router = dev_app(pool.clone());
    clear_syslog(&pool).await;
    dev(&router, "/api/dev/seed-log", json!({ "type": "Error", "title": "over-age", "created": "2020-01-01T00:00:00Z" })).await;
    dev(&router, "/api/dev/seed-log", json!({ "type": "Error", "title": "recent" })).await;

    // graceperiod = 12 months → the 2020 row is over-age; the recent one stays.
    set_cfg(&router, "log_graceperiod", "12").await;
    let resp = dev(&router, "/api/dev/purge-logs", json!({})).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let over: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM syslog WHERE title = 'over-age'").fetch_one(&pool).await.unwrap();
    let rec: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM syslog WHERE title = 'recent'").fetch_one(&pool).await.unwrap();
    assert_eq!(over, 0, "over-age row purged");
    assert_eq!(rec, 1, "recent row kept");

    // graceperiod unset/zero/non-numeric → no-op.
    clear_syslog(&pool).await;
    dev(&router, "/api/dev/seed-log", json!({ "type": "Error", "title": "old2", "created": "2019-01-01T00:00:00Z" })).await;
    for bad in ["0", "abc"] {
        set_cfg(&router, "log_graceperiod", bad).await;
        dev(&router, "/api/dev/purge-logs", json!({})).await;
        let n = syslog_count(&pool).await;
        assert_eq!(n, 1, "no-op sweep when graceperiod={bad}");
    }
}
