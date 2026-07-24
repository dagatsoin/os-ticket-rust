//! Integration tests for the TS-M4-PREP-C/D seed expansion.
//!
//! DB-backed; skipped (pass with a log line) when `TEST_DATABASE_URL` is unset.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p tools --test m4_prep_seed
//! ```
//!
//! @implements TS-M4-PREP-C AC-1: config carries >= 110 keys after seed.
//! @implements TS-M4-PREP-C AC-2: default_dept_id/default_sla_id resolve to real ids.
//! @implements TS-M4-PREP-C AC-3: send_sys_errors stored as a real value.
//! @implements TS-M4-PREP-C AC-4: no duplicate config keys.
//! @implements TS-M4-PREP-C AC-5: admin/Admin123! seeded, isadmin=true, idempotent.
//! @implements TS-M4-PREP-D AC-1..AC-5: email_account/template_group/timezone rows.

use sqlx::postgres::PgPool;
use tools::{ADMIN_PASSWORD, ADMIN_USERNAME, EMAIL_ACCOUNT_ADDR, TEMPLATE_GROUP_NAME};

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

async fn count_where(pool: &PgPool, table: &str, col: &str, val: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table} WHERE {col} = $1"))
        .bind(val)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// TS-M4-PREP-C AC-5: the admin account is seeded with isadmin=true, isactive=true,
/// its argon2id hash verifies, and re-seeding leaves exactly one row (idempotent).
#[tokio::test]
async fn admin_account_seeded_isadmin() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping admin_account_seeded_isadmin");
        return;
    };

    let (isadmin, isactive, passwd): (bool, bool, String) =
        sqlx::query_as("SELECT isadmin, isactive, passwd FROM staff WHERE username = $1")
            .bind(ADMIN_USERNAME)
            .fetch_one(&pool)
            .await
            .expect("admin row must exist");
    assert!(isadmin, "admin.isadmin must be true");
    assert!(isactive, "admin.isactive must be true");
    assert!(passwd.starts_with("$argon2id$"), "admin passwd must be argon2id: {passwd}");
    assert!(
        ost_core::verify_password(ADMIN_PASSWORD, &passwd).unwrap(),
        "admin hash must verify Admin123!"
    );

    // The admin group carries all four net-new M4 capability flags.
    let (faq, premade, ban, stats): (bool, bool, bool, bool) = sqlx::query_as(
        "SELECT can_manage_faq, can_manage_premade, can_ban_emails, can_view_staff_stats
         FROM groups WHERE group_id = (SELECT group_id FROM staff WHERE username = $1)",
    )
    .bind(ADMIN_USERNAME)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(faq && premade && ban && stats, "admin group must carry the 4 M4 flags");

    // Idempotent: a re-seed leaves exactly one admin row.
    tools::seed(&pool).await.expect("re-seed");
    assert_eq!(count_where(&pool, "staff", "username", ADMIN_USERNAME).await, 1);
}

/// TS-M4-PREP-C AC-1/AC-4: config has >= 110 distinct keys and no duplicates.
#[tokio::test]
async fn config_has_110_keys_no_duplicates() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping config_has_110_keys_no_duplicates");
        return;
    };

    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM config")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(count >= 110, "config must carry >= 110 keys, got {count}");

    let dupes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM (SELECT key FROM config GROUP BY key HAVING count(*) > 1) d",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(dupes, 0, "config.key must be unique (no duplicate representations)");

    // Spot-check tab coverage (the 13 keys the AC enumerates).
    for key in [
        "helpdesk_title",
        "helpdesk_url",
        "default_timezone_id",
        "default_ticket_status",
        "default_priority_id",
        "default_help_topic",
        "admin_email",
        "strip_quoted_reply",
        "landing_page_id",
        "offline_page_id",
        "thank-you_page_id",
        "log_level",
        "staff_session_timeout",
    ] {
        assert_eq!(count_where(&pool, "config", "key", key).await, 1, "config `{key}` present");
    }
}

/// TS-M4-PREP-C AC-2 / PREP-D AC-4: the id-valued default bindings resolve to
/// real seeded rows.
#[tokio::test]
async fn default_bindings_resolve_to_real_ids() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping default_bindings_resolve_to_real_ids");
        return;
    };

    // default_dept_id == Support dept_id.
    let dept_ok: bool = sqlx::query_scalar(
        "SELECT (SELECT value::int FROM config WHERE key='default_dept_id')
              = (SELECT dept_id FROM department WHERE dept_name='Support')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(dept_ok, "default_dept_id must equal Support dept_id");

    // default_sla_id references a real SLA row.
    let sla_ok: bool = sqlx::query_scalar(
        "SELECT (SELECT value::int FROM config WHERE key='default_sla_id') IN (SELECT id FROM sla)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(sla_ok, "default_sla_id must reference a real sla row");

    // default_email_id references a real email_account row.
    let email_ok: bool = sqlx::query_scalar(
        "SELECT (SELECT value::int FROM config WHERE key='default_email_id')
              IN (SELECT id FROM email_account)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(email_ok, "default_email_id must reference a real email_account row");

    // default_template_id references a real template_group row.
    let tpl_ok: bool = sqlx::query_scalar(
        "SELECT (SELECT value::int FROM config WHERE key='default_template_id')
              IN (SELECT id FROM template_group)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(tpl_ok, "default_template_id must reference a real template_group row");

    // default_timezone_id references a real timezone row.
    let tz_ok: bool = sqlx::query_scalar(
        "SELECT (SELECT value::int FROM config WHERE key='default_timezone_id')
              IN (SELECT id FROM timezone)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(tz_ok, "default_timezone_id must reference a real timezone row");
}

/// TS-M4-PREP-C AC-3: send_sys_errors is stored as a real value (KL-032.3).
#[tokio::test]
async fn send_sys_errors_stored_as_real_value() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping send_sys_errors_stored_as_real_value");
        return;
    };
    let val: Option<String> = sqlx::query_scalar("SELECT value FROM config WHERE key='send_sys_errors'")
        .fetch_optional(&pool)
        .await
        .unwrap();
    assert_eq!(val.as_deref(), Some("1"), "send_sys_errors must be its seeded real value");
}

/// TS-M4-PREP-D AC-1/2/3/5: the reference rows are seeded (active), and a re-seed
/// does not duplicate them (upsert on the unique key).
#[tokio::test]
async fn reference_rows_seeded_idempotent() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping reference_rows_seeded_idempotent");
        return;
    };

    // Active email_account + template_group present.
    let email_active: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM email_account WHERE email = $1 AND active",
    )
    .bind(EMAIL_ACCOUNT_ADDR)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(email_active, 1, "one active seeded email_account");

    let tpl_active: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM template_group WHERE name = $1 AND isactive",
    )
    .bind(TEMPLATE_GROUP_NAME)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(tpl_active, 1, "one active seeded template_group");

    // Timezone set seeded with a UTC/GMT+0 row.
    let tz_count: i64 = sqlx::query_scalar("SELECT count(*) FROM timezone")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(tz_count > 0, "timezone reference set seeded");
    let utc: i64 = sqlx::query_scalar("SELECT count(*) FROM timezone WHERE gmt_offset = 0")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(utc >= 1, "a UTC/GMT+0 timezone row is present");

    // Idempotent: re-seed keeps each reference row a single instance.
    tools::seed(&pool).await.expect("re-seed");
    assert_eq!(count_where(&pool, "email_account", "email", EMAIL_ACCOUNT_ADDR).await, 1);
    assert_eq!(count_where(&pool, "template_group", "name", TEMPLATE_GROUP_NAME).await, 1);
}

/// TS-M4-PREP-C (test infra): restore_config_defaults force-restores an admin-
/// edited key back to its FS-032 default (the settings-E2E reset path).
#[tokio::test]
async fn restore_config_defaults_overwrites_edits() {
    let Some(pool) = seeded_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping restore_config_defaults_overwrites_edits");
        return;
    };

    // Admin edits a settings key away from its default.
    sqlx::query("UPDATE config SET value = 'Edited Title' WHERE key = 'helpdesk_title'")
        .execute(&pool)
        .await
        .unwrap();

    // A plain re-seed PRESERVES the edit (DO NOTHING) — this is the documented gap.
    tools::seed(&pool).await.expect("re-seed");
    let preserved: String = sqlx::query_scalar("SELECT value FROM config WHERE key='helpdesk_title'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(preserved, "Edited Title", "plain re-seed preserves admin edits");

    // restore_config_defaults force-restores the FS-032 default.
    tools::restore_config_defaults(&pool)
        .await
        .expect("restore config defaults");
    let restored: String = sqlx::query_scalar("SELECT value FROM config WHERE key='helpdesk_title'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(restored, "osTicket Support", "restore_config_defaults resets the key");
}
