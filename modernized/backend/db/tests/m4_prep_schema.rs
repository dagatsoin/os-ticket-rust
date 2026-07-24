//! Integration tests for the TS-M4-PREP-A/B schema migrations (0010 + 0011).
//!
//! DB-backed; skipped (pass with a log line) when `TEST_DATABASE_URL` is unset.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p db --test m4_prep_schema
//! ```
//!
//! @implements TS-M4-PREP-B AC-1: the six net-new tables exist.
//! @implements TS-M4-PREP-B AC-2: page.type / syslog.log_type CHECK constraints.
//! @implements TS-M4-PREP-B AC-3: sla.transient exists (default false).
//! @implements TS-M4-PREP-A AC-1/2/3: department/groups/staff net-new columns.

use sqlx::postgres::PgPool;

fn test_db_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

async fn migrated_pool() -> Option<PgPool> {
    let url = test_db_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn column_exists(pool: &PgPool, table: &str, column: &str) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM information_schema.columns
         WHERE table_name = $1 AND column_name = $2)",
    )
    .bind(table)
    .bind(column)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn table_exists(pool: &PgPool, table: &str) -> bool {
    let reg: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
        .bind(table)
        .fetch_one(pool)
        .await
        .unwrap();
    reg.is_some()
}

/// TS-M4-PREP-B AC-1: the six net-new tables exist.
#[tokio::test]
async fn net_new_tables_exist() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping net_new_tables_exist");
        return;
    };
    for t in [
        "email_account",
        "template_group",
        "timezone",
        "page",
        "faq_category",
        "syslog",
    ] {
        assert!(table_exists(&pool, t).await, "table `{t}` must exist");
    }
    // page carries the expected columns.
    for c in ["type", "body", "isactive", "name"] {
        assert!(column_exists(&pool, "page", c).await, "page.{c} must exist");
    }
    // timezone uses `gmt_offset` (the `offset` reserved word is avoided).
    assert!(column_exists(&pool, "timezone", "gmt_offset").await, "timezone.gmt_offset");
    assert!(column_exists(&pool, "timezone", "dst").await, "timezone.dst");
}

/// TS-M4-PREP-B AC-2: page.type and syslog.log_type reject invalid enum values
/// and accept valid ones.
#[tokio::test]
async fn enum_check_constraints_hold() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping enum_check_constraints_hold");
        return;
    };

    // A bogus page.type is rejected.
    let bad = sqlx::query("INSERT INTO page(name, type, body) VALUES ($1, 'bogus', 'y')")
        .bind(format!("bad-{}", uniq()))
        .execute(&pool)
        .await;
    assert!(bad.is_err(), "invalid page.type must violate the CHECK");

    // A valid page.type is accepted (then cleaned up).
    let ok_name = format!("ok-{}", uniq());
    sqlx::query("INSERT INTO page(name, type, body) VALUES ($1, 'landing', 'y')")
        .bind(&ok_name)
        .execute(&pool)
        .await
        .expect("valid page.type accepted");
    sqlx::query("DELETE FROM page WHERE name = $1")
        .bind(&ok_name)
        .execute(&pool)
        .await
        .unwrap();

    // A bogus syslog.log_type is rejected.
    let bad_log = sqlx::query("INSERT INTO syslog(log_type, title, log) VALUES ('Bogus', 't', 'l')")
        .execute(&pool)
        .await;
    assert!(bad_log.is_err(), "invalid syslog.log_type must violate the CHECK");

    // A valid syslog.log_type is accepted (then cleaned up).
    let title = format!("t-{}", uniq());
    sqlx::query("INSERT INTO syslog(log_type, title, log) VALUES ('Error', $1, 'l')")
        .bind(&title)
        .execute(&pool)
        .await
        .expect("valid syslog.log_type accepted");
    sqlx::query("DELETE FROM syslog WHERE title = $1")
        .bind(&title)
        .execute(&pool)
        .await
        .unwrap();
}

/// TS-M4-PREP-B AC-3: the SLA transient/trump column exists on `sla`, default false.
#[tokio::test]
async fn sla_transient_column_exists_default_false() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping sla_transient_column_exists_default_false");
        return;
    };
    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT data_type, column_default FROM information_schema.columns
         WHERE table_name = 'sla' AND column_name = 'transient'",
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    let (data_type, default) = row.expect("sla.transient must exist");
    assert_eq!(data_type, "boolean", "sla.transient must be boolean");
    assert_eq!(default.as_deref(), Some("false"), "sla.transient default must be false");
}

/// TS-M4-PREP-A AC-1/2/3: the net-new department/groups/staff columns exist.
#[tokio::test]
async fn additive_columns_exist() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping additive_columns_exist");
        return;
    };
    for c in ["email_id", "tpl_id", "manager_id", "group_membership", "autoresp_email_id"] {
        assert!(column_exists(&pool, "department", c).await, "department.{c}");
    }
    for c in ["can_manage_faq", "can_manage_premade", "can_ban_emails", "can_view_staff_stats"] {
        assert!(column_exists(&pool, "groups", c).await, "groups.{c}");
    }
    for c in [
        "onvacation",
        "default_signature_type",
        "default_paper_size",
        "max_page_size",
        "auto_refresh_rate",
        "timezone_id",
        "daylight_saving",
        "change_passwd",
        "passwdreset",
        "notes",
    ] {
        assert!(column_exists(&pool, "staff", c).await, "staff.{c}");
    }

    // The four net-new group flags default false (AC-2).
    for c in ["can_manage_faq", "can_manage_premade", "can_ban_emails", "can_view_staff_stats"] {
        let default: Option<String> = sqlx::query_scalar(
            "SELECT column_default FROM information_schema.columns
             WHERE table_name = 'groups' AND column_name = $1",
        )
        .bind(c)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(default.as_deref(), Some("false"), "groups.{c} default must be false");
    }
}

/// A unique-ish suffix so parallel test runs on the shared DB never collide on
/// the `name`/`title` unique keys.
fn uniq() -> u128 {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static C: AtomicU64 = AtomicU64::new(0);
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    t.wrapping_add(C.fetch_add(1, Ordering::Relaxed) as u128)
}
