//! Idempotent seed fixtures for the M1 slice (TS-M1-A3), M3 expansion (TS-M3-prep),
//! and SLA plans (TS-M3-D2).
//!
//! @implements BS-091: seed fixtures (dept + group + staff) + reference defaults.
//! @implements FS-091.3: status literal `open` reference default.
//! @implements FS-091.5: priority literal `normal` reference default.
//! @implements BS-091.8: group → department access seeded.
//! @implements TS-M3-prep: M3 seed data expansion (second dept, team, help topics, config keys).
//! @implements TS-M3-D2: SLA plan seeding (Default SLA 48h, Standard 24h, Urgent 4h).
//!
//! Modernisation note: this is the stand-in for the legacy installer's
//! default-data seeding (FS-060) and first-run admin bootstrap. It is a Rust
//! task (not pure SQL) because the staff password must be argon2id-hashed with
//! the TS-M1-A4a util, which cannot run inside SQL. Every write is an idempotent
//! upsert, so re-running is the M1 dev-reset mechanism (AC-4).

use ost_core::blob::sha256_hex;
use ost_core::BlobStore;
use sqlx::postgres::PgPool;

/// Seeded department name (M1 primary).
pub const DEPT_NAME: &str = "Support";
/// Seeded second department name (M3 expansion).
/// @implements TS-M3-prep AC-1: second department "Sales".
pub const DEPT_NAME_SALES: &str = "Sales";
/// Seeded permission group name.
pub const GROUP_NAME: &str = "M1 Agents";
/// Seeded staff username (documented QA credential).
pub const STAFF_USERNAME: &str = "agent";
/// Seeded staff plaintext password (documented QA credential).
pub const STAFF_PASSWORD: &str = "Agent123!";

/// Second seeded staff username — the *other* agent for the collaborative
/// lock-conflict E2E (US-M3-I2). Shares the primary agent's group, so it has the
/// same permissions and access to the Support (+ Sales) departments and can open
/// the very same ticket to trigger a "locked by another staff" conflict.
/// @implements FS-021.20: two-agent lock-conflict test infrastructure.
pub const STAFF2_USERNAME: &str = "agent2";
/// Second seeded staff plaintext password (documented QA credential).
pub const STAFF2_PASSWORD: &str = "Agent234!";
/// Second seeded staff first name (drives the lock banner display name).
pub const STAFF2_FIRSTNAME: &str = "Agent";
/// Second seeded staff last name (drives the lock banner display name).
pub const STAFF2_LASTNAME: &str = "Two";
/// FS-091.3 status literal seeded as a config default.
pub const STATUS_OPEN: &str = "open";
/// FS-091.5 priority literal seeded as a config default.
pub const PRIORITY_NORMAL: &str = "normal";

/// Config key holding the default ticket status literal.
pub const CFG_DEFAULT_STATUS: &str = "default_ticket_status";
/// Config key holding the default priority literal.
pub const CFG_DEFAULT_PRIORITY: &str = "default_priority";

// --- TS-M3-prep: M3 seed data expansion constants ---------------------------
//
// @implements TS-M3-prep: second department, team, help topics, and config keys.

/// Seeded team name (M3 expansion).
/// @implements TS-M3-prep AC-2: team "Tier 2".
pub const TEAM_NAME: &str = "Tier 2";

/// Seeded help topic names (M3 expansion).
/// @implements TS-M3-prep AC-3: help topics "General" and "Billing".
pub const TOPIC_GENERAL: &str = "General";
pub const TOPIC_BILLING: &str = "Billing";

/// M3 config keys for queue features.
/// @implements TS-M3-prep AC-5: M3 queue config keys.
pub const CFG_SHOW_ASSIGNED_TICKETS: &str = "show_assigned_tickets";
pub const CFG_SHOW_ANSWERED_TICKETS: &str = "show_answered_tickets";
pub const CFG_TICKET_LOCK_TIME: &str = "ticket_lock_time";
pub const CFG_MAX_PAGE_SIZE: &str = "max_page_size";

/// Pinned default values for M3 config keys.
pub const DEFAULT_SHOW_ASSIGNED_TICKETS: &str = "1";
pub const DEFAULT_SHOW_ANSWERED_TICKETS: &str = "0";
pub const DEFAULT_TICKET_LOCK_TIME: &str = "2";
pub const DEFAULT_MAX_PAGE_SIZE: &str = "25";

// --- TS-M3-D2: SLA plan seed constants -----------------------------------
//
// @implements FS-091.6: Three SLA plans seeded for M3 overdue and queue tests.

/// Name of the default SLA plan (48h grace period, system default).
/// @implements TS-M3-D2 AC-1, AC-2: Default SLA 48h, assigned to config default.
pub const SLA_NAME_DEFAULT: &str = "Default SLA";
/// Grace period (hours) for the default SLA plan.
pub const SLA_GRACE_DEFAULT: i32 = 48;

/// Name of the standard SLA plan (24h grace period).
/// @implements TS-M3-D2 AC-1, AC-3: Standard 24h, assigned to Support department.
pub const SLA_NAME_STANDARD: &str = "Standard";
/// Grace period (hours) for the standard SLA plan.
pub const SLA_GRACE_STANDARD: i32 = 24;

/// Name of the urgent SLA plan (4h grace period).
/// @implements TS-M3-D2 AC-1, AC-4: Urgent 4h, assigned to Billing help topic.
pub const SLA_NAME_URGENT: &str = "Urgent";
/// Grace period (hours) for the urgent SLA plan.
pub const SLA_GRACE_URGENT: i32 = 4;

/// Config key holding the default SLA plan id.
/// @implements TS-M3-D2 AC-2: default_sla_id config key.
pub const CFG_DEFAULT_SLA_ID: &str = "default_sla_id";

// --- TS-M4-PREP-D: reference rows (email_account / template_group / timezone) ---
//
// @implements BS-030-02/03: minimal email + template-group model (DEVIATION M4-D1).
// @implements FS-031.11: timezone reference set for the profile Time Zone select.

/// Seeded default email-account address (the department Email select option).
pub const EMAIL_ACCOUNT_ADDR: &str = "support@osticket.local";
/// Seeded default email-account display name.
pub const EMAIL_ACCOUNT_NAME: &str = "Support";
/// Seeded default template-group name (BS-030-03 "system default" analogue).
pub const TEMPLATE_GROUP_NAME: &str = "osTicket Default";

// --- TS-M4-PREP-C: admin account (blocker for all M4 admin E2E) ---------------
//
// @implements FS-032 (test infra): the `admin`/`Admin123!` isadmin login the M4
//   admin-panel epics authenticate with; M1 only seeds non-admin agent/agent2.

/// Seeded admin group name (carries every permission flag incl. the 4 M4 flags).
pub const ADMIN_GROUP_NAME: &str = "Administrators";
/// Seeded admin staff username (documented QA credential for M4 admin E2E).
pub const ADMIN_USERNAME: &str = "admin";
/// Default admin staff plaintext password (documented QA credential).
///
/// Used only when the `ADMIN_PASSWORD` environment variable is unset — see
/// [`admin_password`]. Production seeds override it by exporting `ADMIN_PASSWORD`
/// for the one seed invocation (the value is never stored in a file).
pub const ADMIN_PASSWORD: &str = "Admin123!";

/// Resolve the admin account password from the environment, falling back to the
/// default [`ADMIN_PASSWORD`] dev credential.
///
/// When the `ADMIN_PASSWORD` env var is set to a non-empty value, that value is
/// used (and argon2id-hashed like any other password). When it is unset or
/// empty, the default `Admin123!` is used — so dev / test behaviour is
/// unchanged and backward-compatible. This lets production seed a strong admin
/// password without baking it into the source or any committed file.
///
/// @implements TS-M4-PREP-C AC-5 (prod hardening): admin password from env.
pub fn admin_password() -> String {
    std::env::var("ADMIN_PASSWORD")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| ADMIN_PASSWORD.to_string())
}

// --- TS-M4-PREP-C: id-valued default bindings (resolve to real seeded ids) -----

/// Config key: default department id (bound to Support).
pub const CFG_DEFAULT_DEPT_ID: &str = "default_dept_id";
/// Config key: default priority id (bound to the `normal` priority, id 2).
pub const CFG_DEFAULT_PRIORITY_ID: &str = "default_priority_id";
/// Config key: default outbound email id (bound to the seeded email_account).
pub const CFG_DEFAULT_EMAIL_ID: &str = "default_email_id";
/// Config key: alerts & notices email id (bound to the seeded email_account).
pub const CFG_ALERT_EMAIL_ID: &str = "alert_email_id";
/// Config key: default template-group id (bound to the seeded template_group).
pub const CFG_DEFAULT_TEMPLATE_ID: &str = "default_template_id";
/// Config key: default timezone id (bound to the seeded UTC timezone row).
pub const CFG_DEFAULT_TIMEZONE_ID: &str = "default_timezone_id";

/// Urgency (priority_id) of the `normal` seeded priority — the default binding.
pub const PRIORITY_NORMAL_ID: i32 = 2;

/// The FS-032 core config keys + FS-032 defaults, seeded as admin-tunable values
/// (`ON CONFLICT (key) DO NOTHING` — a re-seed preserves any admin edit).
///
/// Excluded from this table (handled elsewhere so they are not duplicated):
///   * the 11 M1/M2/M3 keys already seeded (default_ticket_status, default_priority,
///     default_sla_id, allow_attachments, allowed_filetypes, max_file_size,
///     helpdesk_url, show_assigned_tickets, show_answered_tickets, ticket_lock_time,
///     max_page_size);
///   * the id-valued bindings (default_dept_id, default_priority_id, default_email_id,
///     alert_email_id, default_template_id, default_timezone_id) — force-updated to
///     real seeded ids by [`seed_fs032_bindings`].
///
/// @implements FS-032: the ~110 core settings keys the admin tabs read/write.
/// @implements KL-032.3: send_sys_errors stored as its real value (not forced-0).
pub const FS032_CONFIG_DEFAULTS: &[(&str, &str)] = &[
    // --- System / Settings tab ------------------------------------------------
    ("isonline", "1"),
    ("offline_reason", "The helpdesk is offline for maintenance. Please check back later."),
    ("helpdesk_title", "osTicket Support"),
    ("default_locale", "en_US"),
    // Severity LABEL (not numeric): EPIC-M4-G's viewer + the FS-032 System tab
    // agree on this representation; the log writer's parse_log_level gates on it.
    ("log_level", "Debug"),
    ("log_graceperiod", "12"),
    ("passwd_reset_period", "0"),
    ("staff_max_logins", "4"),
    ("staff_login_timeout", "5"),
    ("staff_session_timeout", "30"),
    ("staff_ip_binding", "0"),
    ("client_max_logins", "4"),
    ("client_login_timeout", "1"),
    ("client_session_timeout", "30"),
    ("allow_pw_reset", "1"),
    ("pw_reset_window", "30"),
    ("time_format", "h:i a"),
    ("date_format", "m/d/Y"),
    ("datetime_format", "m/d/Y g:i a"),
    ("daydatetime_format", "D, M j Y g:i a"),
    ("enable_daylight_saving", "0"),
    ("db_tz_offset", "0"),
    ("tz_offset", "0"),
    // --- Ticket tab -----------------------------------------------------------
    ("default_help_topic", "0"),
    ("random_ticket_ids", "1"),
    ("max_open_tickets", "0"),
    ("autolock_minutes", "3"),
    ("allow_priority_change", "0"),
    ("use_email_priority", "0"),
    ("enable_captcha", "0"),
    ("log_ticket_activity", "1"),
    ("auto_assign_reopened_tickets", "1"),
    ("show_related_tickets", "0"),
    ("show_notes_inline", "1"),
    ("clickable_urls", "1"),
    ("hide_staff_name", "0"),
    ("overdue_grace_period", "0"),
    // --- Attachments (system) -------------------------------------------------
    ("max_user_file_uploads", "1"),
    ("max_staff_file_uploads", "1"),
    ("max_file_uploads", "1"),
    ("email_attachments", "1"),
    ("allow_email_attachments", "1"),
    ("allow_online_attachments", "1"),
    ("allow_online_attachments_onlogin", "0"),
    ("allow_api_attachments", "0"),
    // --- Email tab ------------------------------------------------------------
    ("default_smtp_id", "0"),
    ("admin_email", "admin@osticket.local"),
    ("enable_auto_cron", "0"),
    ("enable_mail_polling", "0"),
    ("allow_email_spoofing", "0"),
    ("save_email_headers", "1"),
    ("strip_quoted_reply", "1"),
    ("reply_separator", "-----Reply above this line-----"),
    ("upload_dir", ""),
    ("schema_signature", ""),
    // --- Pages tab (page ids default 0/unbound; bound later in M4-A/F) --------
    ("landing_page_id", "0"),
    ("offline_page_id", "0"),
    ("thank-you_page_id", "0"),
    ("client_logo_id", "0"),
    ("enable_kb", "1"),
    ("enable_premade", "1"),
    // --- Autoresponder tab ----------------------------------------------------
    ("ticket_autoresponder", "1"),
    ("message_autoresponder", "1"),
    ("ticket_notice_active", "1"),
    ("overlimit_notice_active", "1"),
    // --- Alerts & Notices tab -------------------------------------------------
    ("ticket_alert_active", "1"),
    ("ticket_alert_admin", "0"),
    ("ticket_alert_dept_manager", "1"),
    ("ticket_alert_dept_members", "0"),
    ("message_alert_active", "1"),
    ("message_alert_laststaff", "1"),
    ("message_alert_assigned", "1"),
    ("message_alert_dept_manager", "0"),
    ("note_alert_active", "1"),
    ("note_alert_laststaff", "1"),
    ("note_alert_assigned", "1"),
    ("note_alert_dept_manager", "0"),
    ("assigned_alert_active", "1"),
    ("assigned_alert_staff", "1"),
    ("assigned_alert_team_lead", "1"),
    ("assigned_alert_team_members", "1"),
    ("transfer_alert_active", "1"),
    ("transfer_alert_assigned", "1"),
    ("transfer_alert_dept_manager", "1"),
    ("transfer_alert_dept_members", "0"),
    ("overdue_alert_active", "1"),
    ("overdue_alert_assigned", "1"),
    ("overdue_alert_dept_manager", "1"),
    ("overdue_alert_dept_members", "0"),
    ("send_sys_errors", "1"),
    ("send_sql_errors", "1"),
    ("send_login_errors", "1"),
    ("send_mailparse_errors", "1"),
];

// --- TS-M2-A2: attachment + helpdesk config keys -------------------------
//
// @implements FS-022.13 / FS-032 (ref): the attachment config keys the upload
//   gate (TS-M2-A2 `validate_upload`) and the create/reply paths read.
// @implements ROADMAP M2 Decisions §6: `helpdesk_url` base URL for %{url}.

/// Master switch for attachments (FS-022.13). Stored as `"true"`/`"false"`.
pub const CFG_ALLOW_ATTACHMENTS: &str = "allow_attachments";
/// Comma-separated extension allow-list (BS-022.13); `.*` allows all.
pub const CFG_ALLOWED_FILETYPES: &str = "allowed_filetypes";
/// Maximum accepted upload size in bytes (FS-022.13).
pub const CFG_MAX_FILE_SIZE: &str = "max_file_size";
/// Base helpdesk URL the substitution engine reads for `%{url}` (§6).
pub const CFG_HELPDESK_URL: &str = "helpdesk_url";

/// Pinned default: attachments enabled.
pub const DEFAULT_ALLOW_ATTACHMENTS: &str = "true";
/// Pinned default allow-list.
pub const DEFAULT_ALLOWED_FILETYPES: &str = ".pdf,.png,.jpg,.txt,.doc";
/// Pinned default max upload size: 1 MiB.
pub const DEFAULT_MAX_FILE_SIZE: &str = "1048576";
/// Pinned default helpdesk base URL (the Vite SPA origin).
pub const DEFAULT_HELPDESK_URL: &str = "http://localhost:3702";

// --- TS-M2-D1: seeded canned responses -----------------------------------
//
// @implements FS-022.14: the two enabled samples + one disabled sample that
//   stand in for the M4 canned-response CRUD UI. One enabled sample carries
//   `%{...}` variables (exercises the substitution engine, Epic C); one carries
//   a seeded `policy.txt` attachment (exercises blob binding + download).
// @implements BS-022.2: the disabled sample is a negative case for the
//   enabled-only filter.

/// Title of the enabled, variable-carrying canned sample (dept 0 = all).
pub const CANNED_TITLE_ACK: &str = "Acknowledge receipt";
/// Body of the "Acknowledge receipt" sample — carries `%{ticket.number}` and
/// `%{url}` so a fetch substitutes them against the requesting ticket (D2).
pub const CANNED_BODY_ACK: &str = "Hello %{ticket.name},\n\nThanks for contacting support. \
We have received your request and opened ticket #%{ticket.number}. You can follow up at %{url}.\n\n\
Regards,\nThe Support Team";

/// Title of the enabled, attachment-carrying canned sample (dept 0 = all).
pub const CANNED_TITLE_POLICY: &str = "Sample (with attachment)";
/// Body of the attachment-carrying sample (also carries a variable so the
/// substituted-body path is exercised on a response that has an attachment).
pub const CANNED_BODY_POLICY: &str =
    "Please review the attached support policy regarding ticket #%{ticket.number}.";

/// Title of the DISABLED sample (negative case for the enabled-only filter).
pub const CANNED_TITLE_DISABLED: &str = "Closed — disabled sample";
/// Body of the disabled sample (never offered, so plain text is fine).
pub const CANNED_BODY_DISABLED: &str = "This canned response is disabled and must not be offered.";

/// The seeded canned attachment file name.
pub const CANNED_FILE_NAME: &str = "policy.txt";
/// The seeded canned attachment MIME type.
pub const CANNED_FILE_MIME: &str = "text/plain";
/// The seeded canned attachment bytes (stored once via the blob store).
pub const CANNED_FILE_BYTES: &[u8] =
    b"Support Policy\n\nResponses are provided on a best-effort basis during business hours.\n";

/// IDs of the seeded SLA plans, returned for binding to dept/topics.
/// @implements TS-M3-D2: SLA plan seeding result.
#[derive(Debug, Clone, Copy)]
pub struct SlaIds {
    pub default_id: i32,
    pub standard_id: i32,
    pub urgent_id: i32,
}

/// What the seed produced/confirmed, returned for logging and tests.
#[derive(Debug, Clone, Copy)]
pub struct SeedResult {
    pub dept_id: i32,
    pub group_id: i32,
    pub staff_id: i32,
    /// The second agent (`agent2`) seeded for the lock-conflict E2E (US-M3-I2).
    pub staff2_id: i32,
    /// The M4 admin account (`admin`, isadmin) — TS-M4-PREP-C AC-5.
    pub admin_id: i32,
    /// The seeded default email_account id (TS-M4-PREP-D) bound to default_email_id.
    pub email_account_id: i32,
    /// The seeded default template_group id (TS-M4-PREP-D) bound to default_template_id.
    pub template_group_id: i32,
}

/// The seeded M4 reference-row ids (TS-M4-PREP-D), used to resolve the id-valued
/// FS-032 default bindings to real rows.
#[derive(Debug, Clone, Copy)]
pub struct ReferenceIds {
    pub email_account_id: i32,
    pub template_group_id: i32,
    pub timezone_id: i32,
}

/// Seed (or re-confirm) the M1 fixture set + M3 expansion against `pool`, idempotently.
///
/// Upserts departments (Support + Sales), one permission group (with M1 + M3 flags +
/// access to both departments), two staff accounts (`agent` + `agent2`, argon2id-hashed
/// passwords, both in the same group for the lock-conflict E2E), teams, help topics,
/// SLA plans, and config keys. Re-running yields the same rows.
///
/// @implements BS-091: idempotent seed (TS-M1-A3 AC-1..AC-5).
/// @implements TS-M3-prep: M3 seed data expansion (AC-1..AC-6).
/// @implements TS-M3-D2: SLA plan seeding (AC-1..AC-5).
pub async fn seed(pool: &PgPool) -> anyhow::Result<SeedResult> {
    let mut tx = pool.begin().await?;

    // --- department (unique dept_name → ON CONFLICT upsert) ----------------
    // @implements TS-M3-prep AC-1: seed two departments (Support, Sales).
    let dept_id: i32 = sqlx::query_scalar(
        "INSERT INTO department (dept_name, ispublic) VALUES ($1, true)
         ON CONFLICT (dept_name) DO UPDATE SET updated = now()
         RETURNING dept_id",
    )
    .bind(DEPT_NAME)
    .fetch_one(&mut *tx)
    .await?;

    // Second department (M3): Sales
    let sales_dept_id: i32 = sqlx::query_scalar(
        "INSERT INTO department (dept_name, ispublic) VALUES ($1, true)
         ON CONFLICT (dept_name) DO UPDATE SET updated = now()
         RETURNING dept_id",
    )
    .bind(DEPT_NAME_SALES)
    .fetch_one(&mut *tx)
    .await?;

    // --- groups (no unique name column → look up by name, then insert) -----
    // The schema does not enforce group-name uniqueness (FS-091: groups has no
    // name-unique), so emulate upsert-by-name to stay idempotent.
    // @implements TS-M3-prep AC-6: M3 permission flags (can_close/delete/assign/transfer/edit_tickets).
    let existing_group: Option<i32> =
        sqlx::query_scalar("SELECT group_id FROM groups WHERE group_name = $1")
            .bind(GROUP_NAME)
            .fetch_optional(&mut *tx)
            .await?;
    let group_id: i32 = match existing_group {
        Some(id) => {
            sqlx::query(
                "UPDATE groups
                   SET group_enabled = true,
                       can_create_tickets = true,
                       can_post_reply = true,
                       can_close_tickets = true,
                       can_delete_tickets = true,
                       can_assign_tickets = true,
                       can_transfer_tickets = true,
                       can_edit_tickets = true,
                       updated = now()
                 WHERE group_id = $1",
            )
            .bind(id)
            .execute(&mut *tx)
            .await?;
            id
        }
        None => {
            sqlx::query_scalar(
                "INSERT INTO groups
                   (group_name, group_enabled, can_create_tickets, can_post_reply,
                    can_close_tickets, can_delete_tickets, can_assign_tickets,
                    can_transfer_tickets, can_edit_tickets)
                 VALUES ($1, true, true, true, true, true, true, true, true)
                 RETURNING group_id",
            )
            .bind(GROUP_NAME)
            .fetch_one(&mut *tx)
            .await?
        }
    };

    // --- group → department access (composite PK → idempotent upsert) ------
    // @implements TS-M3-prep AC-4: agent's group has access to both Support and Sales.
    sqlx::query(
        "INSERT INTO group_dept_access (group_id, dept_id) VALUES ($1, $2)
         ON CONFLICT (group_id, dept_id) DO NOTHING",
    )
    .bind(group_id)
    .bind(dept_id)
    .execute(&mut *tx)
    .await?;

    // Grant access to Sales department (M3)
    sqlx::query(
        "INSERT INTO group_dept_access (group_id, dept_id) VALUES ($1, $2)
         ON CONFLICT (group_id, dept_id) DO NOTHING",
    )
    .bind(group_id)
    .bind(sales_dept_id)
    .execute(&mut *tx)
    .await?;

    // --- staff (unique username → upsert; hash via TS-M1-A4a) --------------
    let passwd = ost_core::hash_password(STAFF_PASSWORD)?;
    let staff_id: i32 = sqlx::query_scalar(
        "INSERT INTO staff
           (group_id, dept_id, username, firstname, lastname, passwd, email, isactive)
         VALUES ($1, $2, $3, 'Agent', 'One', $4, 'agent@example.com', true)
         ON CONFLICT (username) DO UPDATE
           SET group_id = EXCLUDED.group_id,
               dept_id  = EXCLUDED.dept_id,
               passwd   = EXCLUDED.passwd,
               updated  = now()
         RETURNING staff_id",
    )
    .bind(group_id)
    .bind(dept_id)
    .bind(STAFF_USERNAME)
    .bind(&passwd)
    .fetch_one(&mut *tx)
    .await?;

    // --- second staff (agent2) for the two-agent lock-conflict E2E ----------
    // Shares the primary agent's group (same permissions + Support/Sales access)
    // so it can open the same ticket and trigger a "locked by another staff"
    // conflict. Carries a first/last name so the lock banner shows a display name.
    // @implements FS-021.20: two-agent lock-conflict test infrastructure.
    let passwd2 = ost_core::hash_password(STAFF2_PASSWORD)?;
    let staff2_id: i32 = sqlx::query_scalar(
        "INSERT INTO staff
           (group_id, dept_id, username, firstname, lastname, passwd, email, isactive)
         VALUES ($1, $2, $3, $4, $5, $6, 'agent2@example.com', true)
         ON CONFLICT (username) DO UPDATE
           SET group_id  = EXCLUDED.group_id,
               dept_id   = EXCLUDED.dept_id,
               firstname = EXCLUDED.firstname,
               lastname  = EXCLUDED.lastname,
               passwd    = EXCLUDED.passwd,
               updated   = now()
         RETURNING staff_id",
    )
    .bind(group_id)
    .bind(dept_id)
    .bind(STAFF2_USERNAME)
    .bind(STAFF2_FIRSTNAME)
    .bind(STAFF2_LASTNAME)
    .bind(&passwd2)
    .fetch_one(&mut *tx)
    .await?;

    // --- TS-M4-PREP-D: seed the reference rows (email_account/template_group/
    // timezone) BEFORE the config bindings so default_email_id/default_template_id/
    // default_timezone_id can point at real ids. ---------------------------------
    let ref_ids = seed_reference_rows(&mut tx).await?;

    // Bind the seeded email/template to both departments so the M4-C dept form's
    // Email/Template selects render a real current value (manager_id left NULL).
    // @implements BS-030-02/03: department default email + template bindings.
    sqlx::query(
        "UPDATE department SET email_id = $1, tpl_id = $2, autoresp_email_id = $1, updated = now()
         WHERE dept_id = ANY($3)",
    )
    .bind(ref_ids.email_account_id)
    .bind(ref_ids.template_group_id)
    .bind(&[dept_id, sales_dept_id][..])
    .execute(&mut *tx)
    .await?;

    // --- TS-M4-PREP-C: seed the admin account (isadmin) in a full-capability
    // group carrying the 4 net-new M4 flags + Support/Sales access. -------------
    let admin_id = seed_admin(&mut tx, dept_id, sales_dept_id).await?;

    // --- TS-M3-D2: seed SLA plans and assign to dept/topics -----------------
    // @implements TS-M3-D2 AC-1..AC-4: three SLA plans with dept/topic assignment.
    let sla_ids = seed_sla_plans(&mut tx, dept_id).await?;

    // --- TS-M3-prep AC-2: seed team "Tier 2" with agent as lead + member ---
    seed_team(&mut tx, staff_id).await?;

    // --- TS-M3-prep AC-3: seed help topics (General, Billing) --------------
    // Note: Billing topic gets Urgent SLA per TS-M3-D2 AC-4.
    seed_help_topics(&mut tx, dept_id, sales_dept_id, sla_ids.urgent_id).await?;

    // --- reference defaults the M1 flows read (FS-091 literals) -------------
    upsert_config(&mut tx, CFG_DEFAULT_STATUS, STATUS_OPEN).await?;
    upsert_config(&mut tx, CFG_DEFAULT_PRIORITY, PRIORITY_NORMAL).await?;

    // --- TS-M2-A2: attachment + helpdesk config keys (idempotent) ----------
    seed_m2_config(&mut tx).await?;

    // --- TS-M3-prep AC-5: M3 queue config keys (idempotent) ----------------
    seed_m3_config(&mut tx).await?;

    // --- TS-M4-PREP-C: the ~110 FS-032 core config keys + id-valued bindings -
    seed_fs032_config(&mut tx).await?;
    seed_fs032_bindings(&mut tx, dept_id, &ref_ids).await?;

    tx.commit().await?;

    // --- TS-M2-D1: seeded canned responses (idempotent) --------------------
    // Done after the baseline commit: the attachment-carrying sample stores its
    // `policy.txt` blob on disk (async filesystem work) before binding the
    // relational rows, so it lives in its own step + transaction.
    seed_canned(pool).await?;

    Ok(SeedResult {
        dept_id,
        group_id,
        staff_id,
        staff2_id,
        admin_id,
        email_account_id: ref_ids.email_account_id,
        template_group_id: ref_ids.template_group_id,
    })
}

/// Seed the M4 reference rows (email_account, template_group, timezone),
/// idempotently, returning the ids the config bindings resolve against.
///
/// @implements TS-M4-PREP-D AC-1: one active email_account (support@osticket.local).
/// @implements TS-M4-PREP-D AC-2: one active template_group ("osTicket Default").
/// @implements TS-M4-PREP-D AC-3: the timezone reference set (offset + dst + a UTC row).
/// @implements TS-M4-PREP-D AC-5: idempotent (upsert by unique key, no duplication).
async fn seed_reference_rows(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> anyhow::Result<ReferenceIds> {
    // email_account — upsert by unique email.
    let email_account_id: i32 = sqlx::query_scalar(
        "INSERT INTO email_account (email, name, active) VALUES ($1, $2, true)
         ON CONFLICT (email) DO UPDATE SET name = EXCLUDED.name, active = true, updated = now()
         RETURNING id",
    )
    .bind(EMAIL_ACCOUNT_ADDR)
    .bind(EMAIL_ACCOUNT_NAME)
    .fetch_one(&mut **tx)
    .await?;

    // template_group — upsert by unique name.
    let template_group_id: i32 = sqlx::query_scalar(
        "INSERT INTO template_group (name, isactive) VALUES ($1, true)
         ON CONFLICT (name) DO UPDATE SET isactive = true, updated = now()
         RETURNING id",
    )
    .bind(TEMPLATE_GROUP_NAME)
    .fetch_one(&mut **tx)
    .await?;

    // timezone — a practical offset subset (name is the unique key). The UTC row
    // (gmt_offset 0) backs default_timezone_id and the profile Time Zone select.
    // (offset, name, dst)
    let zones: &[(f64, &str, bool)] = &[
        (-10.0, "(GMT-10:00) Hawaii", false),
        (-8.0, "(GMT-08:00) Pacific Time (US & Canada)", true),
        (-7.0, "(GMT-07:00) Mountain Time (US & Canada)", true),
        (-6.0, "(GMT-06:00) Central Time (US & Canada)", true),
        (-5.0, "(GMT-05:00) Eastern Time (US & Canada)", true),
        (0.0, "(GMT+00:00) UTC/GMT", false),
        (1.0, "(GMT+01:00) Central European Time", true),
        (2.0, "(GMT+02:00) Eastern European Time", true),
        (5.5, "(GMT+05:30) India Standard Time", false),
        (8.0, "(GMT+08:00) China/Singapore", false),
        (9.0, "(GMT+09:00) Japan/Korea", false),
        (10.0, "(GMT+10:00) Sydney", true),
    ];
    let mut utc_id: i32 = 0;
    for (offset, name, dst) in zones {
        let id: i32 = sqlx::query_scalar(
            "INSERT INTO timezone (gmt_offset, name, dst) VALUES ($1, $2, $3)
             ON CONFLICT (name) DO UPDATE SET gmt_offset = EXCLUDED.gmt_offset, dst = EXCLUDED.dst
             RETURNING id",
        )
        .bind(*offset)
        .bind(*name)
        .bind(*dst)
        .fetch_one(&mut **tx)
        .await?;
        if *offset == 0.0 {
            utc_id = id;
        }
    }

    Ok(ReferenceIds {
        email_account_id,
        template_group_id,
        timezone_id: utc_id,
    })
}

/// Seed the `admin`/`Admin123!` isadmin account in a full-capability group
/// (`Administrators`) carrying every M1/M3 flag plus the four net-new M4 flags,
/// with Support + Sales department access. Idempotent (upsert by unique key).
///
/// Blocker for all M4 admin `[BROWSER]` E2E — M1 only seeds non-admin agents.
///
/// @implements TS-M4-PREP-C AC-5: admin login (isadmin=true), idempotent upsert.
async fn seed_admin(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    support_dept_id: i32,
    sales_dept_id: i32,
) -> anyhow::Result<i32> {
    // Administrators group — no unique name column, so emulate upsert-by-name.
    let existing: Option<i32> =
        sqlx::query_scalar("SELECT group_id FROM groups WHERE group_name = $1")
            .bind(ADMIN_GROUP_NAME)
            .fetch_optional(&mut **tx)
            .await?;
    let group_id: i32 = match existing {
        Some(id) => {
            sqlx::query(
                "UPDATE groups SET group_enabled = true, can_create_tickets = true,
                   can_post_reply = true, can_close_tickets = true, can_delete_tickets = true,
                   can_assign_tickets = true, can_transfer_tickets = true, can_edit_tickets = true,
                   can_manage_tickets = true, can_manage_faq = true, can_manage_premade = true,
                   can_ban_emails = true, can_view_staff_stats = true, updated = now()
                 WHERE group_id = $1",
            )
            .bind(id)
            .execute(&mut **tx)
            .await?;
            id
        }
        None => {
            sqlx::query_scalar(
                "INSERT INTO groups
                   (group_name, group_enabled, can_create_tickets, can_post_reply,
                    can_close_tickets, can_delete_tickets, can_assign_tickets,
                    can_transfer_tickets, can_edit_tickets, can_manage_tickets,
                    can_manage_faq, can_manage_premade, can_ban_emails, can_view_staff_stats)
                 VALUES ($1, true, true, true, true, true, true, true, true, true, true, true, true, true)
                 RETURNING group_id",
            )
            .bind(ADMIN_GROUP_NAME)
            .fetch_one(&mut **tx)
            .await?
        }
    };

    // Grant the admin group access to both departments.
    for dept in [support_dept_id, sales_dept_id] {
        sqlx::query(
            "INSERT INTO group_dept_access (group_id, dept_id) VALUES ($1, $2)
             ON CONFLICT (group_id, dept_id) DO NOTHING",
        )
        .bind(group_id)
        .bind(dept)
        .execute(&mut **tx)
        .await?;
    }

    // The admin staff account — upsert by unique username, isadmin=true.
    // Password comes from the `ADMIN_PASSWORD` env var when set (prod), else the
    // default dev credential — see [`admin_password`].
    let passwd = ost_core::hash_password(&admin_password())?;
    let admin_id: i32 = sqlx::query_scalar(
        "INSERT INTO staff
           (group_id, dept_id, username, firstname, lastname, passwd, email, isactive, isadmin)
         VALUES ($1, $2, $3, 'Admin', 'User', $4, 'admin@osticket.local', true, true)
         ON CONFLICT (username) DO UPDATE
           SET group_id = EXCLUDED.group_id,
               dept_id  = EXCLUDED.dept_id,
               passwd   = EXCLUDED.passwd,
               isadmin  = true,
               isactive = true,
               updated  = now()
         RETURNING staff_id",
    )
    .bind(group_id)
    .bind(support_dept_id)
    .bind(ADMIN_USERNAME)
    .bind(&passwd)
    .fetch_one(&mut **tx)
    .await?;

    Ok(admin_id)
}

/// Seed the ~110 FS-032 core config keys to their defaults, idempotently.
///
/// Admin-tunable keys use `ON CONFLICT (key) DO NOTHING` so a re-seed preserves
/// any admin edit (a fresh DB gets the pinned default). The id-valued bindings
/// are force-updated separately by [`seed_fs032_bindings`].
///
/// @implements FS-032: the settings-tab config keys the admin panel reads/writes.
/// @implements KL-032.3: send_sys_errors stored as its real value.
async fn seed_fs032_config(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> anyhow::Result<()> {
    for (key, value) in FS032_CONFIG_DEFAULTS {
        sqlx::query(
            "INSERT INTO config (key, value) VALUES ($1, $2)
             ON CONFLICT (key) DO NOTHING",
        )
        .bind(*key)
        .bind(*value)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Force the id-valued FS-032 default bindings to the ACTUAL seeded row ids
/// (`ON CONFLICT DO UPDATE`), so a reseed always points them at live rows. The
/// `*_page_id` keys default to `0`/unbound (seeded in [`seed_fs032_config`]).
///
/// @implements FS-032: default_dept_id/default_priority_id/default_email_id/
///   alert_email_id/default_template_id/default_timezone_id resolve to real ids.
async fn seed_fs032_bindings(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    support_dept_id: i32,
    ref_ids: &ReferenceIds,
) -> anyhow::Result<()> {
    for (key, id) in [
        (CFG_DEFAULT_DEPT_ID, support_dept_id),
        (CFG_DEFAULT_PRIORITY_ID, PRIORITY_NORMAL_ID),
        (CFG_DEFAULT_EMAIL_ID, ref_ids.email_account_id),
        (CFG_ALERT_EMAIL_ID, ref_ids.email_account_id),
        (CFG_DEFAULT_TEMPLATE_ID, ref_ids.template_group_id),
        (CFG_DEFAULT_TIMEZONE_ID, ref_ids.timezone_id),
    ] {
        sqlx::query(
            "INSERT INTO config (key, value) VALUES ($1, $2)
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated = now()",
        )
        .bind(key)
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Seed the three SLA plans, assign Standard to Support dept, and set default_sla_id.
///
/// @implements TS-M3-D2 AC-1: Three SLA plans (Urgent 4h, Standard 24h, Default SLA 48h).
/// @implements TS-M3-D2 AC-2: Default SLA assigned to config key `default_sla_id`.
/// @implements TS-M3-D2 AC-3: Standard SLA assigned to Support department.
/// @implements TS-M3-D2 AC-5: Idempotent (upsert by unique name).
async fn seed_sla_plans(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    support_dept_id: i32,
) -> anyhow::Result<SlaIds> {
    // Upsert three SLA plans by unique name
    let default_id: i32 = sqlx::query_scalar(
        "INSERT INTO sla (name, grace_period, isactive, enable_priority_escalation, disable_overdue_alerts)
         VALUES ($1, $2, true, true, false)
         ON CONFLICT (name) DO UPDATE
           SET grace_period = EXCLUDED.grace_period,
               isactive = true,
               enable_priority_escalation = true,
               disable_overdue_alerts = false,
               updated = now()
         RETURNING id",
    )
    .bind(SLA_NAME_DEFAULT)
    .bind(SLA_GRACE_DEFAULT)
    .fetch_one(&mut **tx)
    .await?;

    let standard_id: i32 = sqlx::query_scalar(
        "INSERT INTO sla (name, grace_period, isactive, enable_priority_escalation, disable_overdue_alerts)
         VALUES ($1, $2, true, true, false)
         ON CONFLICT (name) DO UPDATE
           SET grace_period = EXCLUDED.grace_period,
               isactive = true,
               enable_priority_escalation = true,
               disable_overdue_alerts = false,
               updated = now()
         RETURNING id",
    )
    .bind(SLA_NAME_STANDARD)
    .bind(SLA_GRACE_STANDARD)
    .fetch_one(&mut **tx)
    .await?;

    let urgent_id: i32 = sqlx::query_scalar(
        "INSERT INTO sla (name, grace_period, isactive, enable_priority_escalation, disable_overdue_alerts)
         VALUES ($1, $2, true, true, false)
         ON CONFLICT (name) DO UPDATE
           SET grace_period = EXCLUDED.grace_period,
               isactive = true,
               enable_priority_escalation = true,
               disable_overdue_alerts = false,
               updated = now()
         RETURNING id",
    )
    .bind(SLA_NAME_URGENT)
    .bind(SLA_GRACE_URGENT)
    .fetch_one(&mut **tx)
    .await?;

    // Assign Standard SLA to Support department
    // @implements TS-M3-D2 AC-3: Support department has Standard SLA.
    sqlx::query("UPDATE department SET sla_id = $1, updated = now() WHERE dept_id = $2")
        .bind(standard_id)
        .bind(support_dept_id)
        .execute(&mut **tx)
        .await?;

    // Upsert default_sla_id config key
    // @implements TS-M3-D2 AC-2: default_sla_id config points to Default SLA.
    sqlx::query(
        "INSERT INTO config (key, value) VALUES ($1, $2)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated = now()",
    )
    .bind(CFG_DEFAULT_SLA_ID)
    .bind(default_id.to_string())
    .execute(&mut **tx)
    .await?;

    Ok(SlaIds {
        default_id,
        standard_id,
        urgent_id,
    })
}

/// Seed the "Tier 2" team with the given staff as lead and member, idempotently.
///
/// @implements TS-M3-prep AC-2: team "Tier 2" with agent as lead + member.
async fn seed_team(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    staff_id: i32,
) -> anyhow::Result<()> {
    // Upsert team by unique name
    let team_id: i32 = sqlx::query_scalar(
        "INSERT INTO team (name, isenabled, lead_id)
         VALUES ($1, true, $2)
         ON CONFLICT (name) DO UPDATE
           SET isenabled = true,
               lead_id = EXCLUDED.lead_id,
               updated = now()
         RETURNING team_id",
    )
    .bind(TEAM_NAME)
    .bind(staff_id)
    .fetch_one(&mut **tx)
    .await?;

    // Add staff as team member (composite PK → idempotent)
    sqlx::query(
        "INSERT INTO team_member (team_id, staff_id) VALUES ($1, $2)
         ON CONFLICT (team_id, staff_id) DO NOTHING",
    )
    .bind(team_id)
    .bind(staff_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// Seed the help topics (General, Billing) with their routing defaults, idempotently.
///
/// @implements TS-M3-prep AC-3: help topics with full column values.
/// @implements TS-M3-D2 AC-4: Billing topic assigned Urgent SLA.
async fn seed_help_topics(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    support_dept_id: i32,
    sales_dept_id: i32,
    urgent_sla_id: i32,
) -> anyhow::Result<()> {
    // "General" topic → Support department (no SLA override — inherits from dept)
    sqlx::query(
        "INSERT INTO help_topic
           (topic, topic_pid, isactive, ispublic, noautoresp, priority_id, dept_id, staff_id, team_id, sla_id, page_id, sort)
         VALUES ($1, 0, true, true, false, 2, $2, NULL, NULL, NULL, 0, 1)
         ON CONFLICT (topic, topic_pid) DO UPDATE
           SET isactive = true,
               ispublic = true,
               noautoresp = false,
               priority_id = 2,
               dept_id = EXCLUDED.dept_id,
               staff_id = NULL,
               team_id = NULL,
               sla_id = NULL,
               page_id = 0,
               sort = 1,
               updated = now()",
    )
    .bind(TOPIC_GENERAL)
    .bind(support_dept_id)
    .execute(&mut **tx)
    .await?;

    // "Billing" topic → Sales department + Urgent SLA
    // @implements TS-M3-D2 AC-4: Billing help topic has Urgent SLA.
    sqlx::query(
        "INSERT INTO help_topic
           (topic, topic_pid, isactive, ispublic, noautoresp, priority_id, dept_id, staff_id, team_id, sla_id, page_id, sort)
         VALUES ($1, 0, true, true, false, 2, $2, NULL, NULL, $3, 0, 2)
         ON CONFLICT (topic, topic_pid) DO UPDATE
           SET isactive = true,
               ispublic = true,
               noautoresp = false,
               priority_id = 2,
               dept_id = EXCLUDED.dept_id,
               staff_id = NULL,
               team_id = NULL,
               sla_id = EXCLUDED.sla_id,
               page_id = 0,
               sort = 2,
               updated = now()",
    )
    .bind(TOPIC_BILLING)
    .bind(sales_dept_id)
    .bind(urgent_sla_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// Seed the M3 queue config keys with their pinned defaults, idempotently.
///
/// @implements TS-M3-prep AC-5: M3 config keys (show_assigned_tickets, show_answered_tickets,
/// ticket_lock_time, max_page_size).
pub async fn seed_m3_config(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> anyhow::Result<()> {
    for (key, value) in [
        (CFG_SHOW_ASSIGNED_TICKETS, DEFAULT_SHOW_ASSIGNED_TICKETS),
        (CFG_SHOW_ANSWERED_TICKETS, DEFAULT_SHOW_ANSWERED_TICKETS),
        (CFG_TICKET_LOCK_TIME, DEFAULT_TICKET_LOCK_TIME),
        (CFG_MAX_PAGE_SIZE, DEFAULT_MAX_PAGE_SIZE),
    ] {
        sqlx::query(
            "INSERT INTO config (key, value) VALUES ($1, $2)
             ON CONFLICT (key) DO NOTHING",
        )
        .bind(key)
        .bind(value)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Seed (or re-confirm) the three canned-response samples, idempotently
/// (TS-M2-D1). Upserts each `canned_response` by its unique `title`, stores the
/// shared `policy.txt` blob once (content-addressed, dedup), upserts its
/// `attachment_file` row by content hash, and binds it to the attachment sample.
///
/// Re-running yields the same three rows and the same single blob (the blob
/// `put` and both upserts are idempotent), preserving the M1 seed contract.
///
/// @implements FS-022.14: the two enabled samples + one disabled sample.
/// @implements BS-022.2: the disabled sample exists for the enabled-only filter.
pub async fn seed_canned(pool: &PgPool) -> anyhow::Result<()> {
    // 1) Store the policy.txt blob once (content-addressed; dedup-idempotent).
    //    The blob root is resolved from BLOB_ROOT (default <cwd>/var/blobs), §1.
    let store = BlobStore::from_env(std::env::current_dir().unwrap_or_else(|_| ".".into()));
    let hash = store.put(CANNED_FILE_BYTES).await?;
    let storage_key = format!("{}/{}/{}", &hash[0..2], &hash[2..4], hash);
    debug_assert_eq!(hash, sha256_hex(CANNED_FILE_BYTES));

    let mut tx = pool.begin().await?;

    // 2) The two enabled samples (dept 0 = all departments) + the disabled one.
    let ack_id = upsert_canned(&mut tx, CANNED_TITLE_ACK, CANNED_BODY_ACK, 0, true).await?;
    let policy_id =
        upsert_canned(&mut tx, CANNED_TITLE_POLICY, CANNED_BODY_POLICY, 0, true).await?;
    let _disabled_id =
        upsert_canned(&mut tx, CANNED_TITLE_DISABLED, CANNED_BODY_DISABLED, 0, false).await?;
    let _ = ack_id; // (no attachment on the ack sample)

    // 3) Upsert the attachment_file row by content hash (D1 dedup), then bind it
    //    to the attachment-carrying sample (idempotent: unique (canned_id, file_id)).
    let file_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO attachment_file (mime, size, hash, name, storage_key)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (hash) DO UPDATE SET name = attachment_file.name
           RETURNING id"#,
    )
    .bind(CANNED_FILE_MIME)
    .bind(CANNED_FILE_BYTES.len() as i64)
    .bind(&hash)
    .bind(CANNED_FILE_NAME)
    .bind(&storage_key)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO canned_attachment (canned_id, file_id) VALUES ($1, $2)
         ON CONFLICT (canned_id, file_id) DO NOTHING",
    )
    .bind(policy_id)
    .bind(file_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

/// Upsert a single canned_response by its unique `title`, returning its id.
///
/// Idempotent: a re-run updates the body/scope/enabled flag in place (so an
/// edited seed re-applies) and never duplicates a row.
async fn upsert_canned(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    title: &str,
    body: &str,
    dept_id: i32,
    isenabled: bool,
) -> anyhow::Result<i32> {
    let id: i32 = sqlx::query_scalar(
        "INSERT INTO canned_response (title, response, dept_id, isenabled)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (title) DO UPDATE
           SET response = EXCLUDED.response,
               dept_id = EXCLUDED.dept_id,
               isenabled = EXCLUDED.isenabled,
               updated = now()
         RETURNING canned_id",
    )
    .bind(title)
    .bind(body)
    .bind(dept_id)
    .bind(isenabled)
    .fetch_one(&mut **tx)
    .await?;
    Ok(id)
}

// --- TS-M2-prep: `--reset` dev purge (truncation half) -------------------
//
// @implements (test infra) TS-M2-prep: a deterministic dev reset that purges
//   ticket-scoped data before re-seeding the baseline, so M2 E2E sweeps start
//   from a clean Open queue. Mechanism is a flag on the seed binary (not an HTTP
//   route). Production-safety: it refuses any DB whose name is not a known dev DB.

/// The only database names `reset` will operate on (production-safety guard).
///
/// `--reset` is destructive (it truncates ticket data), so it refuses to run
/// against anything but these dev/test databases. Verified against the live
/// `current_database()`, not just the DSN, so a mislabelled URL cannot slip past.
pub const ALLOWED_RESET_DBS: [&str; 2] = ["osticket_dev", "osticket_test"];

/// Ticket-scoped tables truncated by `--reset`, in FK-safe (child-first) order.
///
/// PRESERVED by omission: `department`, `groups`, `group_dept_access`, `staff`,
/// `sla`, `config`, the `canned_response` / `canned_attachment` tables (the
/// seeded canned responses + their `policy.txt` blob must survive a reset), and
/// the M4 reference tables `email_account`, `template_group`, `timezone`, `page`,
/// `faq_category`, plus `syslog` (TS-M4-PREP: admin reference/log data survives
/// `--reset`). Only ticket-conversation state is purged here.
///
/// `session` is included so stale staff/client logins don't leak across sweeps;
/// it has no FK to `ticket`, so its position in the list is immaterial.
///
/// NOTE (mailer): recorded mailer sends are NOT persisted in M2 — the StubMailer
/// keeps them in an in-process `Vec`, so there is no DB table to truncate. If a
/// later milestone persists sends, add that table at the head of this list.
pub const RESET_TRUNCATE_TABLES: [&str; 4] = [
    "ticket_attachment", // child of ticket + ticket_thread
    "ticket_thread",     // child of ticket
    "ticket",            // parent
    "session",           // independent login state
];

/// A non-dev target database was refused (production-safety guard tripped).
#[derive(Debug, thiserror::Error)]
#[error(
    "refusing to --reset database `{name}`: not a recognised dev DB \
     (allowed: {allowed})",
    name = .0,
    allowed = ALLOWED_RESET_DBS.join(", ")
)]
pub struct NotADevDatabase(pub String);

/// Assert the connected database is a recognised dev DB, or error.
///
/// Reads the LIVE `current_database()` (authoritative — a mislabelled DSN can't
/// bypass it). Returns the database name on success.
///
/// @implements TS-M2-prep: production-safety guard on the destructive reset.
pub async fn assert_dev_database(pool: &PgPool) -> anyhow::Result<String> {
    let name: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(pool)
        .await?;
    if !ALLOWED_RESET_DBS.contains(&name.as_str()) {
        return Err(NotADevDatabase(name).into());
    }
    Ok(name)
}

/// Destructively reset the dev DB: purge ticket-scoped data, then re-seed.
///
/// Refuses any non-dev database first ([`assert_dev_database`]). Truncates the
/// ticket-conversation tables in FK-safe order inside one transaction (so a
/// failure rolls back to the pre-reset state), then runs the idempotent [`seed`]
/// to restore the dept/group/agent baseline + M2 config keys.
///
/// `TRUNCATE ... CASCADE RESTART IDENTITY` resets the identity sequences too, so
/// a fresh sweep starts ticket numbering from a clean slate.
///
/// Blob reclamation (AC-3): after the re-seed (so the canned `policy.txt` binding
/// is back in place), [`reclaim_orphan_blobs`] prunes every `attachment_file`
/// that is no longer referenced by ANY `ticket_attachment` OR `canned_attachment`
/// row, plus its on-disk blob under `BLOB_ROOT`. The truncation removed all
/// `ticket_attachment` bindings, so the only surviving files are the canned ones
/// — the seeded `policy.txt` is preserved (still bound by `canned_attachment`)
/// while ticket-only blobs are reclaimed.
///
/// @implements TS-M2-prep: ticket-data truncation + blob-reclamation halves of
///   `--reset` (AC-1 purge, AC-2 preserve baseline, AC-3 reclaim orphan blobs).
pub async fn reset(pool: &PgPool) -> anyhow::Result<SeedResult> {
    assert_dev_database(pool).await?;

    let mut tx = pool.begin().await?;
    // One TRUNCATE over all ticket-scoped tables: CASCADE handles any FK edges
    // among them, RESTART IDENTITY rewinds the sequences. The explicit list
    // documents exactly what is purged (and, by omission, what is preserved).
    let stmt = format!(
        "TRUNCATE TABLE {} RESTART IDENTITY CASCADE",
        RESET_TRUNCATE_TABLES.join(", ")
    );
    sqlx::query(&stmt).execute(&mut *tx).await?;
    tx.commit().await?;

    // Restore the baseline (dept/group/agent + reference + M2 config keys + the
    // seeded canned responses, which re-establish the canned policy.txt binding).
    let result = seed(pool).await?;

    // Reclaim now-orphaned blobs (the canned policy.txt is preserved by its
    // canned_attachment reference, re-created by the seed above).
    reclaim_orphan_blobs(pool).await?;

    Ok(result)
}

/// Prune every `attachment_file` no longer referenced by ANY `ticket_attachment`
/// OR `canned_attachment` row, removing both the metadata row and its on-disk
/// blob under `BLOB_ROOT`. Returns the number of files reclaimed.
///
/// Called by [`reset`] after the re-seed: a file is an orphan exactly when no
/// binding of either kind points at it. The seeded canned `policy.txt` is never
/// an orphan (the re-seeded `canned_attachment` binds it), so it survives; a
/// ticket-only file (all of whose `ticket_attachment` bindings were truncated)
/// is reclaimed.
///
/// Blob removal is idempotent ([`BlobStore::remove`] treats a missing file as
/// success), and the DB row is deleted in the same pass; a blob shared by
/// another still-referenced row is never touched because that row would not be
/// in the orphan set. Resolved against `BLOB_ROOT` (§1) like every binary.
///
/// @implements TS-M2-prep (AC-3): blob/file reclamation after the purge,
///   preserving still-referenced (canned) blobs.
pub async fn reclaim_orphan_blobs(pool: &PgPool) -> anyhow::Result<u64> {
    // Find orphan files: referenced by neither a ticket nor a canned binding.
    let orphans: Vec<(i64, String)> = sqlx::query_as(
        "SELECT af.id, af.hash
         FROM attachment_file af
         WHERE NOT EXISTS (SELECT 1 FROM ticket_attachment ta WHERE ta.file_id = af.id)
           AND NOT EXISTS (SELECT 1 FROM canned_attachment ca WHERE ca.file_id = af.id)",
    )
    .fetch_all(pool)
    .await?;

    if orphans.is_empty() {
        return Ok(0);
    }

    let store = BlobStore::from_env(std::env::current_dir().unwrap_or_else(|_| ".".into()));
    let mut reclaimed = 0u64;
    for (id, hash) in orphans {
        // Remove the on-disk blob first (idempotent), then the metadata row.
        // Only remove the blob when no OTHER attachment_file row shares this hash
        // (the hash is UNIQUE, so this orphan owns its blob exclusively — but the
        // guard keeps the invariant explicit and safe if that ever changes).
        let shared: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM attachment_file WHERE hash = $1 AND id <> $2",
        )
        .bind(&hash)
        .bind(id)
        .fetch_one(pool)
        .await?;
        if shared == 0 {
            store.remove(&hash).await?;
        }
        sqlx::query("DELETE FROM attachment_file WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        reclaimed += 1;
    }
    Ok(reclaimed)
}

/// Force-restore the FS-032 config defaults (admin-tunable keys + id bindings),
/// overwriting any admin edit, so a settings-tab E2E can return to a known state.
///
/// Unlike [`seed_fs032_config`] (which preserves existing values via DO NOTHING),
/// this force-updates every FS-032 default (DO UPDATE) and re-points the id-valued
/// bindings at the live seeded rows (Support dept, seeded email/template/timezone).
/// It also restores the M2/M3 admin-tunable defaults. Backs the dev
/// `POST /api/dev/reset-config` endpoint (M4-A settings E2E reset path).
///
/// @implements TS-M4-PREP-C (test infra): config-reset path for the settings E2E.
pub async fn restore_config_defaults(pool: &PgPool) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;

    // Force-update every FS-032 default + the M2/M3 admin-tunable defaults.
    let forced: Vec<(&str, &str)> = FS032_CONFIG_DEFAULTS
        .iter()
        .copied()
        .chain([
            (CFG_ALLOW_ATTACHMENTS, DEFAULT_ALLOW_ATTACHMENTS),
            (CFG_ALLOWED_FILETYPES, DEFAULT_ALLOWED_FILETYPES),
            (CFG_MAX_FILE_SIZE, DEFAULT_MAX_FILE_SIZE),
            (CFG_HELPDESK_URL, DEFAULT_HELPDESK_URL),
            (CFG_SHOW_ASSIGNED_TICKETS, DEFAULT_SHOW_ASSIGNED_TICKETS),
            (CFG_SHOW_ANSWERED_TICKETS, DEFAULT_SHOW_ANSWERED_TICKETS),
            (CFG_TICKET_LOCK_TIME, DEFAULT_TICKET_LOCK_TIME),
            (CFG_MAX_PAGE_SIZE, DEFAULT_MAX_PAGE_SIZE),
            (CFG_DEFAULT_STATUS, STATUS_OPEN),
            (CFG_DEFAULT_PRIORITY, PRIORITY_NORMAL),
        ])
        .collect();
    for (key, value) in forced {
        upsert_config(&mut tx, key, value).await?;
    }

    // Re-point the id-valued bindings at the live seeded rows (best-effort: only
    // when the referenced rows exist).
    let support_dept_id: Option<i32> =
        sqlx::query_scalar("SELECT dept_id FROM department WHERE dept_name = $1")
            .bind(DEPT_NAME)
            .fetch_optional(&mut *tx)
            .await?;
    let email_account_id: Option<i32> =
        sqlx::query_scalar("SELECT id FROM email_account WHERE email = $1")
            .bind(EMAIL_ACCOUNT_ADDR)
            .fetch_optional(&mut *tx)
            .await?;
    let template_group_id: Option<i32> =
        sqlx::query_scalar("SELECT id FROM template_group WHERE name = $1")
            .bind(TEMPLATE_GROUP_NAME)
            .fetch_optional(&mut *tx)
            .await?;
    let timezone_id: Option<i32> =
        sqlx::query_scalar("SELECT id FROM timezone WHERE gmt_offset = 0 ORDER BY id LIMIT 1")
            .fetch_optional(&mut *tx)
            .await?;
    let default_sla_id: Option<i32> =
        sqlx::query_scalar("SELECT id FROM sla WHERE name = $1")
            .bind(SLA_NAME_DEFAULT)
            .fetch_optional(&mut *tx)
            .await?;

    let mut bindings: Vec<(&str, i32)> = vec![(CFG_DEFAULT_PRIORITY_ID, PRIORITY_NORMAL_ID)];
    if let Some(id) = support_dept_id {
        bindings.push((CFG_DEFAULT_DEPT_ID, id));
    }
    if let Some(id) = email_account_id {
        bindings.push((CFG_DEFAULT_EMAIL_ID, id));
        bindings.push((CFG_ALERT_EMAIL_ID, id));
    }
    if let Some(id) = template_group_id {
        bindings.push((CFG_DEFAULT_TEMPLATE_ID, id));
    }
    if let Some(id) = timezone_id {
        bindings.push((CFG_DEFAULT_TIMEZONE_ID, id));
    }
    if let Some(id) = default_sla_id {
        bindings.push((CFG_DEFAULT_SLA_ID, id));
    }
    for (key, id) in bindings {
        upsert_config(&mut tx, key, &id.to_string()).await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Seed the four M2 config keys with their pinned defaults, idempotently.
///
/// `INSERT ... ON CONFLICT (key) DO NOTHING` so a re-run leaves the existing
/// value untouched (and never duplicates a row) — these are admin-tunable
/// defaults, not reference literals to overwrite. A fresh DB gets the pinned
/// values; an existing DB keeps whatever an admin has since set.
///
/// @implements FS-022.13 / ROADMAP §6: attachment + helpdesk config seed (AC-5).
pub async fn seed_m2_config(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> anyhow::Result<()> {
    for (key, value) in [
        (CFG_ALLOW_ATTACHMENTS, DEFAULT_ALLOW_ATTACHMENTS),
        (CFG_ALLOWED_FILETYPES, DEFAULT_ALLOWED_FILETYPES),
        (CFG_MAX_FILE_SIZE, DEFAULT_MAX_FILE_SIZE),
        (CFG_HELPDESK_URL, DEFAULT_HELPDESK_URL),
    ] {
        sqlx::query(
            "INSERT INTO config (key, value) VALUES ($1, $2)
             ON CONFLICT (key) DO NOTHING",
        )
        .bind(key)
        .bind(value)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Idempotently upsert a single config key/value pair.
async fn upsert_config(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    key: &str,
    value: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO config (key, value) VALUES ($1, $2)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated = now()",
    )
    .bind(key)
    .bind(value)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `admin_password` falls back to the default dev credential when the
    /// `ADMIN_PASSWORD` env var is unset or empty (backward-compatible), and
    /// returns the env value when it is set (production override).
    ///
    /// @implements TS-M4-PREP-C AC-5 (prod hardening): admin password from env.
    #[test]
    fn admin_password_uses_env_override_else_default() {
        // Serialised into one test because the env var is process-global.
        std::env::remove_var("ADMIN_PASSWORD");
        assert_eq!(admin_password(), ADMIN_PASSWORD, "unset → default");

        std::env::set_var("ADMIN_PASSWORD", "");
        assert_eq!(admin_password(), ADMIN_PASSWORD, "empty → default");

        std::env::set_var("ADMIN_PASSWORD", "S3cret-Prod-Pw!");
        assert_eq!(admin_password(), "S3cret-Prod-Pw!", "set → override");

        std::env::remove_var("ADMIN_PASSWORD");
        assert_eq!(admin_password(), ADMIN_PASSWORD, "cleaned up → default");
    }
}
