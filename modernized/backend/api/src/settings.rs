//! Admin System Settings endpoints (TS-M4-A1) — FS-032.
//!
//! Two admin-gated routes back the settings panel:
//!
//! * `GET  /api/staff/admin/settings` — every config key grouped into the eight
//!   tabs the UI renders, plus the option lists the selects need (FS-032.2).
//! * `PUT  /api/staff/admin/settings` — save one tab at a time: run only that
//!   tab's validators, resolve checkbox keys by presence, and upsert only that
//!   tab's allowlisted keys transactionally (FS-032.2 / FS-032.7 / BS-032.x).
//!
//! The single source of truth is [`KEYS`]: a static key→tab map that also flags
//! each key as a checkbox (presence-valued 0/1, BS-032.4) and/or a non-negative
//! integer field. Both the GET bucketing and the PUT allowlist derive from it, so
//! a submitted tab can never write another tab's keys (FS-032.7).
//!
//! @implements FS-032.1: administrator-only gate (per-route, via require_admin).
//! @implements FS-032.2: read grouped config + per-tab validation on write.
//! @implements FS-032.7: a save touches only the submitted tab's keys.
//! @implements BS-032.4: checkbox keys persist 0/1 by presence.
//! @implements BS-032.5: an enabled alert event requires ≥1 recipient.
//! @implements BS-032.6: attachment sub-field validation is conditional.
//! @implements BS-032.7: admin_email must be a valid, non-system email.
//! @implements BS-032.8: strip_quoted_reply requires a non-empty reply_separator.
//! @implements KL-032.3: send_sys_errors persists its real value (modernised).

use std::collections::{BTreeMap, HashMap};

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use sqlx::postgres::PgPool;

use ost_core::validation::is_email;
use ost_core::{is_known_log_level, ApiError};

use crate::auth::gate::require_admin;
use crate::auth::realm::StaffSession;
use crate::auth::StaffCsrf;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Tab model + the key→tab map (single source of truth).
// ---------------------------------------------------------------------------

/// The eight settings tabs (names pinned by the TS-M4-A0 frontend contract).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    System,
    Tickets,
    Emails,
    Pages,
    Kb,
    Autoresp,
    Alerts,
    Attach,
}

impl Tab {
    /// All tabs, in the order the GET payload lists them.
    pub const ALL: [Tab; 8] = [
        Tab::System,
        Tab::Tickets,
        Tab::Emails,
        Tab::Pages,
        Tab::Kb,
        Tab::Autoresp,
        Tab::Alerts,
        Tab::Attach,
    ];

    /// Wire name of the tab.
    pub fn as_str(self) -> &'static str {
        match self {
            Tab::System => "system",
            Tab::Tickets => "tickets",
            Tab::Emails => "emails",
            Tab::Pages => "pages",
            Tab::Kb => "kb",
            Tab::Autoresp => "autoresp",
            Tab::Alerts => "alerts",
            Tab::Attach => "attach",
        }
    }

    /// Parse a tab name; `None` for an unknown tab (→ 422 at the route).
    pub fn from_name(s: &str) -> Option<Tab> {
        Tab::ALL.into_iter().find(|t| t.as_str() == s)
    }
}

/// One config key's placement + input kind.
struct KeyDef {
    key: &'static str,
    tab: Tab,
    /// Presence-valued 0/1 checkbox (BS-032.4).
    checkbox: bool,
    /// Must parse as a non-negative integer when present.
    numeric: bool,
}

const fn kd(key: &'static str, tab: Tab, checkbox: bool, numeric: bool) -> KeyDef {
    KeyDef {
        key,
        tab,
        checkbox,
        numeric,
    }
}

/// The static key→tab map. Every seeded FS-032 config key appears exactly once;
/// this drives both GET bucketing and the PUT allowlist (FS-032.7).
#[rustfmt::skip]
const KEYS: &[KeyDef] = &[
    // --- system --------------------------------------------------------------
    kd("isonline", Tab::System, true, false),
    kd("offline_reason", Tab::System, false, false),
    kd("helpdesk_title", Tab::System, false, false),
    kd("helpdesk_name", Tab::System, false, false),
    kd("default_locale", Tab::System, false, false),
    // log_level is a severity LABEL (Error/Warning/Debug), not a numeric field —
    // validated against the known set by `validate_log_level` (EPIC-M4-G / FS-033.1).
    kd("log_level", Tab::System, false, false),
    kd("log_graceperiod", Tab::System, false, true),
    kd("passwd_reset_period", Tab::System, false, true),
    kd("staff_max_logins", Tab::System, false, true),
    kd("staff_login_timeout", Tab::System, false, true),
    kd("staff_session_timeout", Tab::System, false, true),
    kd("staff_ip_binding", Tab::System, true, false),
    kd("client_max_logins", Tab::System, false, true),
    kd("client_login_timeout", Tab::System, false, true),
    kd("client_session_timeout", Tab::System, false, true),
    kd("allow_pw_reset", Tab::System, true, false),
    kd("pw_reset_window", Tab::System, false, true),
    kd("time_format", Tab::System, false, false),
    kd("date_format", Tab::System, false, false),
    kd("datetime_format", Tab::System, false, false),
    kd("daydatetime_format", Tab::System, false, false),
    kd("enable_daylight_saving", Tab::System, true, false),
    kd("db_tz_offset", Tab::System, false, false),
    kd("tz_offset", Tab::System, false, false),
    kd("default_timezone_id", Tab::System, false, false),
    kd("helpdesk_url", Tab::System, false, false),
    kd("max_page_size", Tab::System, false, true),
    // --- tickets -------------------------------------------------------------
    kd("default_help_topic", Tab::Tickets, false, true),
    kd("random_ticket_ids", Tab::Tickets, true, false),
    kd("max_open_tickets", Tab::Tickets, false, true),
    kd("autolock_minutes", Tab::Tickets, false, true),
    kd("allow_priority_change", Tab::Tickets, true, false),
    kd("use_email_priority", Tab::Tickets, true, false),
    kd("enable_captcha", Tab::Tickets, true, false),
    kd("log_ticket_activity", Tab::Tickets, true, false),
    kd("auto_assign_reopened_tickets", Tab::Tickets, true, false),
    kd("show_related_tickets", Tab::Tickets, true, false),
    kd("show_notes_inline", Tab::Tickets, true, false),
    kd("clickable_urls", Tab::Tickets, true, false),
    kd("hide_staff_name", Tab::Tickets, true, false),
    kd("overdue_grace_period", Tab::Tickets, false, true),
    kd("default_priority_id", Tab::Tickets, false, true),
    kd("default_sla_id", Tab::Tickets, false, true),
    kd("default_dept_id", Tab::Tickets, false, true),
    kd("default_ticket_status", Tab::Tickets, false, false),
    kd("default_priority", Tab::Tickets, false, false),
    kd("show_assigned_tickets", Tab::Tickets, true, false),
    kd("show_answered_tickets", Tab::Tickets, true, false),
    kd("ticket_lock_time", Tab::Tickets, false, true),
    // --- emails --------------------------------------------------------------
    kd("default_smtp_id", Tab::Emails, false, false),
    kd("admin_email", Tab::Emails, false, false),
    kd("enable_auto_cron", Tab::Emails, true, false),
    kd("enable_mail_polling", Tab::Emails, true, false),
    kd("allow_email_spoofing", Tab::Emails, true, false),
    kd("save_email_headers", Tab::Emails, true, false),
    kd("strip_quoted_reply", Tab::Emails, true, false),
    kd("reply_separator", Tab::Emails, false, false),
    kd("upload_dir", Tab::Emails, false, false),
    kd("schema_signature", Tab::Emails, false, false),
    kd("default_email_id", Tab::Emails, false, false),
    kd("alert_email_id", Tab::Emails, false, false),
    kd("default_template_id", Tab::Emails, false, false),
    // --- pages ---------------------------------------------------------------
    kd("landing_page_id", Tab::Pages, false, false),
    kd("offline_page_id", Tab::Pages, false, false),
    kd("thank-you_page_id", Tab::Pages, false, false),
    kd("client_logo_id", Tab::Pages, false, false),
    // --- kb (knowledgebase) --------------------------------------------------
    kd("enable_kb", Tab::Kb, true, false),
    kd("enable_premade", Tab::Kb, true, false),
    // --- autoresp ------------------------------------------------------------
    kd("ticket_autoresponder", Tab::Autoresp, true, false),
    kd("message_autoresponder", Tab::Autoresp, true, false),
    kd("ticket_notice_active", Tab::Autoresp, true, false),
    kd("overlimit_notice_active", Tab::Autoresp, true, false),
    // --- alerts (all checkboxes) ---------------------------------------------
    kd("ticket_alert_active", Tab::Alerts, true, false),
    kd("ticket_alert_admin", Tab::Alerts, true, false),
    kd("ticket_alert_dept_manager", Tab::Alerts, true, false),
    kd("ticket_alert_dept_members", Tab::Alerts, true, false),
    kd("message_alert_active", Tab::Alerts, true, false),
    kd("message_alert_laststaff", Tab::Alerts, true, false),
    kd("message_alert_assigned", Tab::Alerts, true, false),
    kd("message_alert_dept_manager", Tab::Alerts, true, false),
    kd("note_alert_active", Tab::Alerts, true, false),
    kd("note_alert_laststaff", Tab::Alerts, true, false),
    kd("note_alert_assigned", Tab::Alerts, true, false),
    kd("note_alert_dept_manager", Tab::Alerts, true, false),
    kd("assigned_alert_active", Tab::Alerts, true, false),
    kd("assigned_alert_staff", Tab::Alerts, true, false),
    kd("assigned_alert_team_lead", Tab::Alerts, true, false),
    kd("assigned_alert_team_members", Tab::Alerts, true, false),
    kd("transfer_alert_active", Tab::Alerts, true, false),
    kd("transfer_alert_assigned", Tab::Alerts, true, false),
    kd("transfer_alert_dept_manager", Tab::Alerts, true, false),
    kd("transfer_alert_dept_members", Tab::Alerts, true, false),
    kd("overdue_alert_active", Tab::Alerts, true, false),
    kd("overdue_alert_assigned", Tab::Alerts, true, false),
    kd("overdue_alert_dept_manager", Tab::Alerts, true, false),
    kd("overdue_alert_dept_members", Tab::Alerts, true, false),
    kd("send_sys_errors", Tab::Alerts, true, false),
    kd("send_sql_errors", Tab::Alerts, true, false),
    kd("send_login_errors", Tab::Alerts, true, false),
    kd("send_mailparse_errors", Tab::Alerts, true, false),
    // --- attach --------------------------------------------------------------
    kd("allow_attachments", Tab::Attach, true, false),
    kd("email_attachments", Tab::Attach, true, false),
    kd("allow_email_attachments", Tab::Attach, true, false),
    kd("allow_online_attachments", Tab::Attach, true, false),
    kd("allow_online_attachments_onlogin", Tab::Attach, true, false),
    kd("allow_api_attachments", Tab::Attach, true, false),
    kd("max_user_file_uploads", Tab::Attach, false, true),
    kd("max_staff_file_uploads", Tab::Attach, false, true),
    kd("max_file_uploads", Tab::Attach, false, true),
    kd("max_file_size", Tab::Attach, false, true),
    kd("allowed_filetypes", Tab::Attach, false, false),
];

/// The six alert events (BS-032.5): `(active_key, recipient_keys)`. When the
/// active flag is on, at least one recipient sub-flag must also be on.
#[rustfmt::skip]
const ALERT_EVENTS: &[(&str, &[&str])] = &[
    ("ticket_alert_active", &["ticket_alert_admin", "ticket_alert_dept_manager", "ticket_alert_dept_members"]),
    ("message_alert_active", &["message_alert_laststaff", "message_alert_assigned", "message_alert_dept_manager"]),
    ("note_alert_active", &["note_alert_laststaff", "note_alert_assigned", "note_alert_dept_manager"]),
    ("assigned_alert_active", &["assigned_alert_staff", "assigned_alert_team_lead", "assigned_alert_team_members"]),
    ("transfer_alert_active", &["transfer_alert_assigned", "transfer_alert_dept_manager", "transfer_alert_dept_members"]),
    ("overdue_alert_active", &["overdue_alert_assigned", "overdue_alert_dept_manager", "overdue_alert_dept_members"]),
];

// ---------------------------------------------------------------------------
// Value coercion helpers.
// ---------------------------------------------------------------------------

/// Whether a submitted JSON value counts as "checked" for a checkbox key.
///
/// Presence with a truthy value → on; an explicit falsy value (0/"0"/false/"")
/// → off; absent → off (handled by the caller). This is a superset of the
/// pinned "absent→0, present→1" rule that also honours an explicit `0`
/// (AC-7 submits `allow_attachments: 0` to mean OFF).
fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Value::String(s) => matches!(
            s.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "on" | "yes"
        ),
        _ => false,
    }
}

/// Coerce a submitted JSON value to its stored string form. `Null` → skip.
fn value_to_string(v: &Value) -> Option<String> {
    match v {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        Value::Bool(b) => Some(if *b { "1" } else { "0" }.to_string()),
        Value::Number(n) => Some(n.to_string()),
        other => Some(other.to_string()),
    }
}

/// Resolve the submitted values into the concrete `{key: stored_string}` set the
/// tab would persist. Checkbox keys always resolve (present→truthy 1/0, absent→0,
/// BS-032.4). Non-checkbox keys resolve only when submitted.
fn resolve_tab_values(tab: Tab, values: &Map<String, Value>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for def in KEYS.iter().filter(|d| d.tab == tab) {
        if def.checkbox {
            let on = values.get(def.key).map(is_truthy).unwrap_or(false);
            out.insert(def.key.to_string(), if on { "1" } else { "0" }.to_string());
        } else if let Some(v) = values.get(def.key) {
            if let Some(s) = value_to_string(v) {
                out.insert(def.key.to_string(), s);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Validators. Each populates `errors` (field → message); DB errors bubble up.
// ---------------------------------------------------------------------------

/// Validate every non-negative-integer key of `tab` present in `resolved`.
fn validate_numeric_keys(tab: Tab, resolved: &BTreeMap<String, String>, errors: &mut BTreeMap<String, String>) {
    for def in KEYS.iter().filter(|d| d.tab == tab && d.numeric) {
        if let Some(s) = resolved.get(def.key) {
            let t = s.trim();
            if !t.is_empty() && t.parse::<u64>().is_err() {
                errors.insert(def.key.to_string(), "Enter a positive number".to_string());
            }
        }
    }
}

/// The System tab's `log_level` is a severity LABEL (Error/Warning/Debug), not a
/// numeric field — reject a value outside the known set (→ 422 field). Accepts
/// exactly what `ost_core::log::parse_log_level` gates on, so the settings save
/// and the log-gating read agree on one representation (EPIC-M4-G / FS-033.1).
///
/// @implements FS-032.2: System-tab field validation (log_level severity label).
/// @implements FS-033.1: log_level shares one representation with the log writer.
fn validate_log_level(resolved: &BTreeMap<String, String>, errors: &mut BTreeMap<String, String>) {
    if let Some(s) = resolved.get("log_level") {
        let t = s.trim();
        if !t.is_empty() && !is_known_log_level(t) {
            errors.insert("log_level".to_string(), "Select a valid log level".to_string());
        }
    }
}

/// BS-032.5: an enabled alert event with no selected recipient is a field error
/// keyed on the event's `_active` flag.
fn validate_alerts(resolved: &BTreeMap<String, String>, errors: &mut BTreeMap<String, String>) {
    for (active, recips) in ALERT_EVENTS {
        let on = resolved.get(*active).map(|s| s == "1").unwrap_or(false);
        if on {
            let any = recips
                .iter()
                .any(|r| resolved.get(*r).map(|s| s == "1").unwrap_or(false));
            if !any {
                errors.insert((*active).to_string(), "Select recipient(s)".to_string());
            }
        }
    }
}

/// BS-032.6: attachment sub-fields are only validated while the master switch is
/// on (`allow_attachments == 1`).
fn validate_attach(resolved: &BTreeMap<String, String>, errors: &mut BTreeMap<String, String>) {
    let allow = resolved.get("allow_attachments").map(|s| s == "1").unwrap_or(false);
    if allow {
        validate_numeric_keys(Tab::Attach, resolved, errors);
    }
}

/// BS-032.7 + BS-032.8: admin_email shape/collision + reply_separator dependency.
async fn validate_emails(
    pool: &PgPool,
    resolved: &BTreeMap<String, String>,
    errors: &mut BTreeMap<String, String>,
) -> Result<(), ApiError> {
    if let Some(email) = resolved.get("admin_email") {
        let e = email.trim();
        if e.is_empty() {
            errors.insert("admin_email".to_string(), "This field is required".to_string());
        } else if !is_email(e) {
            errors.insert("admin_email".to_string(), "Enter a valid email address".to_string());
        } else {
            // BS-032.7: the admin address must not collide with a system email.
            let clash: Option<i32> = sqlx::query_scalar(
                "SELECT 1 FROM email_account WHERE lower(email) = lower($1) LIMIT 1",
            )
            .bind(e)
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::internal("Email validation failed"))?;
            if clash.is_some() {
                errors.insert(
                    "admin_email".to_string(),
                    "Cannot be a system/department email address".to_string(),
                );
            }
        }
    }

    // BS-032.8: if quoting is stripped, the separator marker must be non-empty.
    let strip = resolved.get("strip_quoted_reply").map(|s| s == "1").unwrap_or(false);
    if strip {
        let sep_ok = resolved
            .get("reply_separator")
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !sep_ok {
            errors.insert("reply_separator".to_string(), "This field is required".to_string());
        }
    }
    Ok(())
}

/// Run only the submitted tab's validator(s) over the resolved values.
async fn validate_tab(
    pool: &PgPool,
    tab: Tab,
    resolved: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, ApiError> {
    let mut errors = BTreeMap::new();
    match tab {
        Tab::System => {
            validate_numeric_keys(tab, resolved, &mut errors);
            validate_log_level(resolved, &mut errors);
        }
        Tab::Tickets => validate_numeric_keys(tab, resolved, &mut errors),
        Tab::Emails => validate_emails(pool, resolved, &mut errors).await?,
        Tab::Alerts => validate_alerts(resolved, &mut errors),
        Tab::Attach => validate_attach(resolved, &mut errors),
        // pages / kb / autoresp: presence-only, no cross-field validation.
        Tab::Pages | Tab::Kb | Tab::Autoresp => {}
    }
    Ok(errors)
}

// ---------------------------------------------------------------------------
// GET /api/staff/admin/settings
// ---------------------------------------------------------------------------

/// `GET /api/staff/admin/settings` — grouped config + option lists (admin only).
///
/// @implements FS-032.2: read every config key grouped by tab.
/// @implements FS-032.1: admin-gated (401 no session / 403 non-admin).
pub async fn get_settings(
    State(state): State<AppState>,
    session: StaffSession,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Settings are unavailable"))?;

    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM config")
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::internal("Settings lookup failed"))?;
    let cfg: HashMap<String, String> = rows.into_iter().collect();

    // Bucket each mapped, present key into its tab. Missing keys are omitted.
    let mut tabs = Map::new();
    for tab in Tab::ALL {
        let mut obj = Map::new();
        for def in KEYS.iter().filter(|d| d.tab == tab) {
            if let Some(v) = cfg.get(def.key) {
                obj.insert(def.key.to_string(), Value::String(v.clone()));
            }
        }
        tabs.insert(tab.as_str().to_string(), Value::Object(obj));
    }

    let options = load_options(pool).await?;

    Ok(Json(json!({ "tabs": Value::Object(tabs), "options": options })))
}

/// Build the inline option lists the settings selects need (FS-032.2).
async fn load_options(pool: &PgPool) -> Result<Value, ApiError> {
    let err = |_e| ApiError::internal("Option lookup failed");

    let departments: Vec<(i32, String)> =
        sqlx::query_as("SELECT dept_id, dept_name FROM department ORDER BY dept_name")
            .fetch_all(pool)
            .await
            .map_err(err)?;
    let sla_plans: Vec<(i32, String)> =
        sqlx::query_as("SELECT id, name FROM sla ORDER BY name")
            .fetch_all(pool)
            .await
            .map_err(err)?;
    let help_topics: Vec<(i32, String)> =
        sqlx::query_as("SELECT topic_id, topic FROM help_topic ORDER BY sort, topic")
            .fetch_all(pool)
            .await
            .map_err(err)?;
    let priorities: Vec<(i32, String)> =
        sqlx::query_as("SELECT priority_id, priority_desc FROM priority ORDER BY urgency")
            .fetch_all(pool)
            .await
            .map_err(err)?;
    let email_accounts: Vec<(i32, String, String)> =
        sqlx::query_as("SELECT id, email, name FROM email_account ORDER BY email")
            .fetch_all(pool)
            .await
            .map_err(err)?;
    let template_groups: Vec<(i32, String)> =
        sqlx::query_as("SELECT id, name FROM template_group ORDER BY name")
            .fetch_all(pool)
            .await
            .map_err(err)?;
    let timezones: Vec<(i32, String)> =
        sqlx::query_as("SELECT id, name FROM timezone ORDER BY gmt_offset, id")
            .fetch_all(pool)
            .await
            .map_err(err)?;
    // Site pages back the Pages-tab selects (landing/offline/thank-you page ids).
    let pages: Vec<(i32, String)> =
        sqlx::query_as("SELECT id, name FROM page ORDER BY name")
            .fetch_all(pool)
            .await
            .map_err(err)?;

    let id_name = |rows: Vec<(i32, String)>| -> Value {
        Value::Array(
            rows.into_iter()
                .map(|(id, name)| json!({ "id": id, "name": name }))
                .collect(),
        )
    };

    Ok(json!({
        "departments": id_name(departments),
        "sla_plans": id_name(sla_plans),
        "help_topics": id_name(help_topics),
        "priorities": id_name(priorities),
        "email_accounts": Value::Array(
            email_accounts
                .into_iter()
                .map(|(id, email, name)| json!({ "id": id, "email": email, "name": name }))
                .collect(),
        ),
        "template_groups": id_name(template_groups),
        "timezones": Value::Array(
            timezones
                .into_iter()
                .map(|(id, label)| json!({ "id": id, "label": label }))
                .collect(),
        ),
        "pages": id_name(pages),
    }))
}

// ---------------------------------------------------------------------------
// PUT /api/staff/admin/settings
// ---------------------------------------------------------------------------

/// The PUT body: which tab, and the submitted values for it.
#[derive(Debug, Deserialize)]
pub struct PutSettings {
    pub tab: String,
    #[serde(default)]
    pub values: Map<String, Value>,
}

/// `PUT /api/staff/admin/settings` — save one tab (admin only, CSRF-enforced).
///
/// Validates only the submitted tab (FS-032.2), resolves checkbox keys by
/// presence (BS-032.4), and upserts only that tab's allowlisted keys inside one
/// transaction (FS-032.7), skipping no-op writes.
///
/// @implements FS-032.2: per-tab validation + 422 field-error envelope.
/// @implements FS-032.7: only the submitted tab's keys are written.
/// @implements BS-032.4: checkbox keys persist 0/1 by presence.
pub async fn put_settings(
    State(state): State<AppState>,
    csrf: StaffCsrf,
    Json(body): Json<PutSettings>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &csrf.0).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Settings are unavailable"))?;

    // Reject an unknown tab (422) — nothing is written.
    let tab = Tab::from_name(&body.tab)
        .ok_or_else(|| ApiError::validation(format!("Unknown settings tab: {}", body.tab)))?;

    let resolved = resolve_tab_values(tab, &body.values);

    // Run only this tab's validators; any field error rolls the whole save back.
    let errors = validate_tab(pool, tab, &resolved).await?;
    if !errors.is_empty() {
        return Err(ApiError::validation("Please correct the errors below").with_fields(errors));
    }

    // Transactional upsert with a no-op skip: only changed keys are written, and
    // only this tab's keys are ever touched (FS-032.7).
    let keys: Vec<String> = resolved.keys().cloned().collect();
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal("Settings save failed"))?;

    let current_rows: Vec<(String, String)> =
        sqlx::query_as("SELECT key, value FROM config WHERE key = ANY($1)")
            .bind(&keys)
            .fetch_all(&mut *tx)
            .await
            .map_err(|_| ApiError::internal("Settings save failed"))?;
    let current: HashMap<String, String> = current_rows.into_iter().collect();

    let mut saved = 0usize;
    for (key, value) in &resolved {
        if current.get(key) == Some(value) {
            continue; // no-op skip
        }
        sqlx::query(
            "INSERT INTO config (key, value) VALUES ($1, $2) \
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated = now()",
        )
        .bind(key)
        .bind(value)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("Settings save failed"))?;
        saved += 1;
    }

    tx.commit()
        .await
        .map_err(|_| ApiError::internal("Settings save failed"))?;

    // Echo the stored values for the tab.
    let echoed: Map<String, Value> = resolved
        .into_iter()
        .map(|(k, v)| (k, Value::String(v)))
        .collect();

    Ok(Json(json!({
        "tab": tab.as_str(),
        "saved": saved,
        "values": Value::Object(echoed),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, Value)]) -> Map<String, Value> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    #[test]
    fn every_key_is_mapped_once() {
        let mut seen = std::collections::HashSet::new();
        for def in KEYS {
            assert!(seen.insert(def.key), "duplicate key in map: {}", def.key);
        }
    }

    #[test]
    fn tab_names_round_trip() {
        for t in Tab::ALL {
            assert_eq!(Tab::from_name(t.as_str()), Some(t));
        }
        assert_eq!(Tab::from_name("nope"), None);
    }

    #[test]
    fn checkbox_presence_resolves_0_or_1() {
        // Present truthy → 1; explicit 0 → 0; absent → 0 (BS-032.4).
        let v = map(&[("ticket_alert_active", json!(1))]);
        let r = resolve_tab_values(Tab::Alerts, &v);
        assert_eq!(r.get("ticket_alert_active").unwrap(), "1");
        assert_eq!(r.get("ticket_alert_admin").unwrap(), "0"); // absent → 0

        let v = map(&[("allow_attachments", json!(0))]);
        let r = resolve_tab_values(Tab::Attach, &v);
        assert_eq!(r.get("allow_attachments").unwrap(), "0"); // explicit 0 → 0
    }

    #[test]
    fn numeric_field_rejects_non_integer() {
        let v = map(&[("max_open_tickets", json!("abc"))]);
        let r = resolve_tab_values(Tab::Tickets, &v);
        let mut errs = BTreeMap::new();
        validate_numeric_keys(Tab::Tickets, &r, &mut errs);
        assert!(errs.contains_key("max_open_tickets"));
    }

    #[test]
    fn log_level_label_is_not_a_numeric_field() {
        // A severity LABEL must NOT trip the positive-number validator (the AC-7
        // cross-epic bug: the seeded "Debug" broke every System-tab save).
        let v = map(&[("log_level", json!("Debug"))]);
        let r = resolve_tab_values(Tab::System, &v);
        let mut errs = BTreeMap::new();
        validate_numeric_keys(Tab::System, &r, &mut errs);
        assert!(!errs.contains_key("log_level"), "log_level is not numeric");

        // The label validator accepts the known set (label or legacy numeric).
        for ok in ["Error", "Warning", "Debug", "3"] {
            let v = map(&[("log_level", json!(ok))]);
            let r = resolve_tab_values(Tab::System, &v);
            let mut errs = BTreeMap::new();
            validate_log_level(&r, &mut errs);
            assert!(errs.is_empty(), "{ok} is a valid log_level");
        }

        // An unknown value is a 422 field error.
        let v = map(&[("log_level", json!("Loud"))]);
        let r = resolve_tab_values(Tab::System, &v);
        let mut errs = BTreeMap::new();
        validate_log_level(&r, &mut errs);
        assert!(errs.contains_key("log_level"), "unknown log_level rejected");
    }

    #[test]
    fn alert_matrix_requires_recipient() {
        // ticket_alert_active on, all recipients omitted → error (BS-032.5).
        let v = map(&[("ticket_alert_active", json!(1))]);
        let r = resolve_tab_values(Tab::Alerts, &v);
        let mut errs = BTreeMap::new();
        validate_alerts(&r, &mut errs);
        assert_eq!(errs.get("ticket_alert_active").unwrap(), "Select recipient(s)");

        // With a recipient selected, no error.
        let v = map(&[("ticket_alert_active", json!(1)), ("ticket_alert_admin", json!(1))]);
        let r = resolve_tab_values(Tab::Alerts, &v);
        let mut errs = BTreeMap::new();
        validate_alerts(&r, &mut errs);
        assert!(errs.is_empty());
    }

    #[test]
    fn attach_validation_is_conditional() {
        // allow off → negative size ignored (BS-032.6).
        let v = map(&[("allow_attachments", json!(0)), ("max_file_size", json!("-5"))]);
        let r = resolve_tab_values(Tab::Attach, &v);
        let mut errs = BTreeMap::new();
        validate_attach(&r, &mut errs);
        assert!(errs.is_empty());

        // allow on → negative size rejected.
        let v = map(&[("allow_attachments", json!(1)), ("max_file_size", json!("-5"))]);
        let r = resolve_tab_values(Tab::Attach, &v);
        let mut errs = BTreeMap::new();
        validate_attach(&r, &mut errs);
        assert!(errs.contains_key("max_file_size"));
    }
}
