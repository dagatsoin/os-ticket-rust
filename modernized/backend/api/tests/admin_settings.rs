//! Integration tests for TS-M4-A1 — admin System Settings endpoints + the
//! isadmin/capability loader on `GET /api/staff/me`.
//!
//! Driven through the router via `oneshot`; every test skips (pass + log line)
//! when `TEST_DATABASE_URL` is unset so DB-less CI stays green.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p api --test admin_settings
//! ```
//!
//! @implements FS-032.1: /api/staff/me isadmin + 4 capability flags; require_admin gate.
//! @implements FS-032.2: GET grouped settings + options; PUT per-tab validation.
//! @implements FS-032.7: PUT writes only the submitted tab's keys.
//! @implements BS-032.4/.5/.6: checkbox presence, alert-matrix, attachment conditional.
//! @implements KL-032.3: send_sys_errors round-trips its real value.

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
    // Force config back to the FS-032 defaults so tests are deterministic even
    // after a previous run mutated keys.
    tools::restore_config_defaults(&pool).await.expect("reset config");
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

/// Log in, returning (session-cookie value, csrf-token value).
async fn login(router: &Router, user: &str, pass: &str) -> (String, String) {
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
    assert_eq!(resp.status(), StatusCode::OK, "login {user} should succeed");
    let sess = set_cookie_value(&resp, "ost_staff_sess").expect("session cookie");
    let csrf = set_cookie_value(&resp, "XSRF-TOKEN-STAFF").expect("csrf cookie");
    (sess, csrf)
}

async fn get_settings(router: &Router, sess: &str) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/staff/admin/settings")
                .header(COOKIE, format!("ost_staff_sess={sess}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn put_settings(
    router: &Router,
    sess: &str,
    csrf: &str,
    body: Value,
) -> http::Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/staff/admin/settings")
                .header(
                    COOKIE,
                    format!("ost_staff_sess={sess}; XSRF-TOKEN-STAFF={csrf}"),
                )
                .header(CSRF_HEADER, csrf)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

macro_rules! skip_if_no_db {
    ($name:expr) => {{
        let Some(pool) = seeded_pool().await else {
            eprintln!("TEST_DATABASE_URL unset — skipping {}", $name);
            return;
        };
        pool
    }};
}

// --- AC-8: /api/staff/me exposes isadmin + the four capability flags ----------

#[tokio::test]
async fn me_admin_has_isadmin_and_capability_flags() {
    let pool = skip_if_no_db!("me_admin_has_isadmin_and_capability_flags");
    let router = dev_app(pool);

    let (sess, _) = login(&router, "admin", "Admin123!").await;
    let me = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/staff/me")
                .header(COOKIE, format!("ost_staff_sess={sess}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(me.status(), StatusCode::OK);
    let j = json_body(me).await;
    assert_eq!(j["isadmin"], true, "admin isadmin=true");
    assert_eq!(j["can_manage_faq"], true);
    assert_eq!(j["can_manage_premade"], true);
    assert_eq!(j["can_ban_emails"], true);
    assert_eq!(j["can_view_staff_stats"], true);
}

#[tokio::test]
async fn me_agent_has_isadmin_false_and_flags_false() {
    let pool = skip_if_no_db!("me_agent_has_isadmin_false_and_flags_false");
    let router = dev_app(pool);

    let (sess, _) = login(&router, "agent", "Agent123!").await;
    let me = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/staff/me")
                .header(COOKIE, format!("ost_staff_sess={sess}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(me.status(), StatusCode::OK);
    let j = json_body(me).await;
    assert_eq!(j["isadmin"], false, "plain agent isadmin=false");
    assert_eq!(j["can_manage_faq"], false);
    assert_eq!(j["can_manage_premade"], false);
    assert_eq!(j["can_ban_emails"], false);
    assert_eq!(j["can_view_staff_stats"], false);
}

// --- AC-9: require_admin gates GET/PUT (403 non-admin / 401 no session) -------

#[tokio::test]
async fn settings_get_403_for_agent_and_401_without_session() {
    let pool = skip_if_no_db!("settings_get_403_for_agent_and_401_without_session");
    let router = dev_app(pool);

    let (agent_sess, _) = login(&router, "agent", "Agent123!").await;
    let resp = get_settings(&router, &agent_sess).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "agent GET → 403");

    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/staff/admin/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "no session GET → 401");
}

#[tokio::test]
async fn settings_put_403_for_agent_writes_nothing() {
    let pool = skip_if_no_db!("settings_put_403_for_agent_writes_nothing");
    let router = dev_app(pool);

    let (admin_sess, _) = login(&router, "admin", "Admin123!").await;
    // Record helpdesk_title before the rejected non-admin PUT.
    let before = json_body(get_settings(&router, &admin_sess).await).await;
    let title_before = before["tabs"]["system"]["helpdesk_title"].clone();

    let (agent_sess, agent_csrf) = login(&router, "agent", "Agent123!").await;
    let resp = put_settings(
        &router,
        &agent_sess,
        &agent_csrf,
        json!({ "tab": "system", "values": { "helpdesk_title": "HACKED" } }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "agent PUT → 403");

    let after = json_body(get_settings(&router, &admin_sess).await).await;
    assert_eq!(
        after["tabs"]["system"]["helpdesk_title"], title_before,
        "rejected non-admin PUT wrote nothing"
    );
}

// --- AC-1 / AC-10: GET returns the eight tabs + option lists ------------------

#[tokio::test]
async fn get_returns_all_tabs_and_options() {
    let pool = skip_if_no_db!("get_returns_all_tabs_and_options");
    let router = dev_app(pool);

    // Seed a landing page so options.pages is non-empty for the Site Pages selects.
    let seed = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/dev/seed-page")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "type": "landing" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(seed.status(), StatusCode::CREATED);

    let (sess, _) = login(&router, "admin", "Admin123!").await;
    let j = json_body(get_settings(&router, &sess).await).await;

    for tab in [
        "system", "tickets", "emails", "pages", "kb", "autoresp", "alerts", "attach",
    ] {
        assert!(j["tabs"][tab].is_object(), "tab {tab} present");
    }
    // kb tab holds enable_kb + enable_premade.
    assert!(j["tabs"]["kb"]["enable_kb"].is_string());
    assert!(j["tabs"]["kb"]["enable_premade"].is_string());
    // A representative key in each of a few tabs.
    assert!(j["tabs"]["tickets"]["max_open_tickets"].is_string());
    assert!(j["tabs"]["alerts"]["ticket_alert_active"].is_string());

    for opt in [
        "departments", "sla_plans", "help_topics", "priorities", "email_accounts",
        "template_groups", "timezones", "pages",
    ] {
        assert!(j["options"][opt].is_array(), "option list {opt} present");
        assert!(
            !j["options"][opt].as_array().unwrap().is_empty(),
            "option list {opt} non-empty from seed"
        );
    }
    // email_accounts carries email + name; timezones carry label.
    assert!(j["options"]["email_accounts"][0]["email"].is_string());
    assert!(j["options"]["timezones"][0]["label"].is_string());
    assert!(j["options"]["priorities"][0]["name"].is_string());
    // pages carry {id, name} for the Site Pages selects.
    assert!(j["options"]["pages"][0]["id"].is_i64() || j["options"]["pages"][0]["id"].is_u64());
    assert!(j["options"]["pages"][0]["name"].is_string());
}

// --- AC-2 + FS-032.7: PUT saves the submitted tab; other tab unchanged --------

#[tokio::test]
async fn put_saves_tickets_field_and_leaves_system_untouched() {
    let pool = skip_if_no_db!("put_saves_tickets_field_and_leaves_system_untouched");
    let router = dev_app(pool);

    let (sess, csrf) = login(&router, "admin", "Admin123!").await;
    let before = json_body(get_settings(&router, &sess).await).await;
    let title_before = before["tabs"]["system"]["helpdesk_title"].clone();

    // priorities option list gives a valid priority id.
    let pid = before["options"]["priorities"][0]["id"].as_i64().unwrap();
    let resp = put_settings(
        &router,
        &sess,
        &csrf,
        json!({ "tab": "tickets", "values": { "default_priority_id": pid } }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let saved = json_body(resp).await;
    assert_eq!(saved["tab"], "tickets");
    assert_eq!(saved["values"]["default_priority_id"], pid.to_string());

    let after = json_body(get_settings(&router, &sess).await).await;
    assert_eq!(
        after["tabs"]["tickets"]["default_priority_id"],
        pid.to_string(),
        "ticket default_priority_id changed"
    );
    assert_eq!(
        after["tabs"]["system"]["helpdesk_title"], title_before,
        "system helpdesk_title unchanged by a tickets save"
    );
}

// --- AC-3: non-numeric value → 422 field error, nothing written --------------

#[tokio::test]
async fn put_non_numeric_returns_422_field_error() {
    let pool = skip_if_no_db!("put_non_numeric_returns_422_field_error");
    let router = dev_app(pool);

    let (sess, csrf) = login(&router, "admin", "Admin123!").await;
    let before = json_body(get_settings(&router, &sess).await).await;
    let moc_before = before["tabs"]["tickets"]["max_open_tickets"].clone();

    let resp = put_settings(
        &router,
        &sess,
        &csrf,
        json!({ "tab": "tickets", "values": { "max_open_tickets": "abc" } }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let j = json_body(resp).await;
    assert!(
        j["error"]["fields"]["max_open_tickets"].is_string(),
        "field error on max_open_tickets"
    );

    let after = json_body(get_settings(&router, &sess).await).await;
    assert_eq!(
        after["tabs"]["tickets"]["max_open_tickets"], moc_before,
        "rejected write is atomic — value unchanged"
    );
}

// --- unknown tab → 422 -------------------------------------------------------

#[tokio::test]
async fn put_unknown_tab_returns_422() {
    let pool = skip_if_no_db!("put_unknown_tab_returns_422");
    let router = dev_app(pool);
    let (sess, csrf) = login(&router, "admin", "Admin123!").await;
    let resp = put_settings(
        &router,
        &sess,
        &csrf,
        json!({ "tab": "nonsense", "values": { "x": 1 } }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// --- AC-4: checkbox persists 0/1 by presence ---------------------------------

#[tokio::test]
async fn alert_checkbox_persists_zero_then_one_by_presence() {
    let pool = skip_if_no_db!("alert_checkbox_persists_zero_then_one_by_presence");
    let router = dev_app(pool);
    let (sess, csrf) = login(&router, "admin", "Admin123!").await;

    // Event validly configured (ticket_alert_active on, dept_manager recipient on),
    // but OMIT ticket_alert_admin → it should persist as 0.
    let resp = put_settings(
        &router,
        &sess,
        &csrf,
        json!({ "tab": "alerts", "values": {
            "ticket_alert_active": 1,
            "ticket_alert_dept_manager": 1
        }}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let after = json_body(get_settings(&router, &sess).await).await;
    assert_eq!(after["tabs"]["alerts"]["ticket_alert_admin"], "0", "omitted → 0");

    // Now INCLUDE ticket_alert_admin on → persists 1.
    let resp = put_settings(
        &router,
        &sess,
        &csrf,
        json!({ "tab": "alerts", "values": {
            "ticket_alert_active": 1,
            "ticket_alert_dept_manager": 1,
            "ticket_alert_admin": 1
        }}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let after = json_body(get_settings(&router, &sess).await).await;
    assert_eq!(after["tabs"]["alerts"]["ticket_alert_admin"], "1", "present → 1");
}

// --- AC-5: send_sys_errors round-trips its real value (KL-032.3) --------------

#[tokio::test]
async fn send_sys_errors_round_trips_real_value() {
    let pool = skip_if_no_db!("send_sys_errors_round_trips_real_value");
    let router = dev_app(pool);
    let (sess, csrf) = login(&router, "admin", "Admin123!").await;

    let resp = put_settings(
        &router,
        &sess,
        &csrf,
        json!({ "tab": "alerts", "values": { "send_sys_errors": 1 } }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let after = json_body(get_settings(&router, &sess).await).await;
    assert_eq!(after["tabs"]["alerts"]["send_sys_errors"], "1");
}

// --- AC-6: enabled alert event with no recipients → 422 (BS-032.5) -----------

#[tokio::test]
async fn enabled_alert_without_recipient_returns_422() {
    let pool = skip_if_no_db!("enabled_alert_without_recipient_returns_422");
    let router = dev_app(pool);
    let (sess, csrf) = login(&router, "admin", "Admin123!").await;

    let resp = put_settings(
        &router,
        &sess,
        &csrf,
        json!({ "tab": "alerts", "values": { "ticket_alert_active": 1 } }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let j = json_body(resp).await;
    assert_eq!(
        j["error"]["fields"]["ticket_alert_active"], "Select recipient(s)",
        "BS-032.5 field error"
    );
}

// --- AC-7: attachment sub-field validation is conditional (BS-032.6) ---------

#[tokio::test]
async fn attachment_validation_is_conditional_on_master_switch() {
    let pool = skip_if_no_db!("attachment_validation_is_conditional_on_master_switch");
    let router = dev_app(pool);
    let (sess, csrf) = login(&router, "admin", "Admin123!").await;

    // Master switch OFF → invalid size ignored → 200.
    let resp = put_settings(
        &router,
        &sess,
        &csrf,
        json!({ "tab": "attach", "values": { "allow_attachments": 0, "max_file_size": "-5" } }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "skipped while master off");

    // Master switch ON → invalid size rejected → 422 field error.
    let resp = put_settings(
        &router,
        &sess,
        &csrf,
        json!({ "tab": "attach", "values": { "allow_attachments": 1, "max_file_size": "-5" } }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let j = json_body(resp).await;
    assert!(j["error"]["fields"]["max_file_size"].is_string());
}

// --- Cross-epic (M4 root E2E AC-7): log_level is a severity LABEL, not numeric.
// The frontend re-submits the FULL raw System tab (log_level included); a save
// that carries the seeded "Debug" must SUCCEED and persist the page-size fields.

#[tokio::test]
async fn system_tab_save_with_log_level_label_persists_page_size() {
    let pool = skip_if_no_db!("system_tab_save_with_log_level_label_persists_page_size");
    let router = dev_app(pool);
    let (sess, csrf) = login(&router, "admin", "Admin123!").await;

    let before = json_body(get_settings(&router, &sess).await).await;
    // The seed stores a severity label (settings + the log writer agree on it).
    let log_level = before["tabs"]["system"]["log_level"]
        .as_str()
        .expect("log_level present")
        .to_string();
    assert_eq!(log_level, "Debug", "seed stores the Debug severity label");

    // Re-submit the full System tab (as AdminSettingsStore does), changing page
    // size + password-reset period while carrying the seeded log_level.
    let resp = put_settings(
        &router,
        &sess,
        &csrf,
        json!({ "tab": "system", "values": {
            "log_level": log_level,
            "max_page_size": 5,
            "passwd_reset_period": 7
        }}),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "System save carrying a label log_level succeeds (was 422 before the fix)"
    );

    let after = json_body(get_settings(&router, &sess).await).await;
    assert_eq!(after["tabs"]["system"]["max_page_size"], "5", "page size persisted");
    assert_eq!(
        after["tabs"]["system"]["passwd_reset_period"], "7",
        "password reset period persisted"
    );
    assert_eq!(
        after["tabs"]["system"]["log_level"], "Debug",
        "log_level severity label round-trips unchanged"
    );
}

#[tokio::test]
async fn system_tab_rejects_unknown_log_level() {
    let pool = skip_if_no_db!("system_tab_rejects_unknown_log_level");
    let router = dev_app(pool);
    let (sess, csrf) = login(&router, "admin", "Admin123!").await;

    let resp = put_settings(
        &router,
        &sess,
        &csrf,
        json!({ "tab": "system", "values": { "log_level": "Loud", "max_page_size": 9 } }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let j = json_body(resp).await;
    assert!(
        j["error"]["fields"]["log_level"].is_string(),
        "unknown log_level → field error"
    );

    // Atomic: the co-submitted max_page_size must NOT have persisted.
    let after = json_body(get_settings(&router, &sess).await).await;
    assert_ne!(after["tabs"]["system"]["max_page_size"], "9", "rejected save wrote nothing");
}

// --- seed-page dev endpoint inserts a page -----------------------------------

#[tokio::test]
async fn dev_seed_page_inserts_a_page() {
    let pool = skip_if_no_db!("dev_seed_page_inserts_a_page");
    let router = dev_app(pool.clone());

    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/dev/seed-page")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "type": "landing" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let j = json_body(resp).await;
    let id = j["id"].as_i64().expect("page id returned");

    let (kind,): (String,) = sqlx::query_as("SELECT type FROM page WHERE id = $1")
        .bind(id as i32)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(kind, "landing");
}
