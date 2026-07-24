//! Integration tests for the TS-M1-A2 migrations.
//!
//! These connect to a live Postgres test database. They are skipped (pass with
//! a log line) when `TEST_DATABASE_URL` is not set, so the offline CI build and
//! a DB-less `cargo test` both stay green. Locally / in DB-enabled CI:
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p db --test migrations
//! ```
//!
//! @implements BS-091: PostgreSQL migrations for the M1 schema subset (TS-M1-A2).

use sqlx::postgres::PgPool;
use sqlx::Row;

/// Read the test DB URL, or `None` (test becomes a no-op) when unset.
fn test_db_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

async fn migrated_pool() -> Option<PgPool> {
    let url = test_db_url()?;
    let pool = db::connect(&url).await.expect("connect to test DB");
    db::migrate(&pool).await.expect("first migrate");
    Some(pool)
}

/// AC-1 / AC-2: migrations apply, and a second run is a safe no-op.
#[tokio::test]
async fn migrations_apply_and_are_idempotent() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping migrations_apply_and_are_idempotent");
        return;
    };
    // Re-running must not error and must not re-apply (idempotency).
    db::migrate(&pool).await.expect("second migrate is a no-op");
}

/// AC-3: the FS-091-aligned tables exist, including `groups` (not `group`).
#[tokio::test]
async fn expected_tables_exist() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping expected_tables_exist");
        return;
    };
    for table in [
        "department",
        "sla",
        "groups",
        "group_dept_access",
        "staff",
        "ticket",
        "ticket_thread",
        "session",
        "config",
    ] {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables \
             WHERE table_schema='public' AND table_name=$1)",
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(exists, "expected table `{table}` to exist");
    }
    // The reserved word must NOT have been used.
    let reserved: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables \
         WHERE table_schema='public' AND table_name='group')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!reserved, "a table named `group` must not exist");
}

/// AC-4: dual identifiers + UNIQUE (ticketID, email) is enforced.
#[tokio::test]
async fn ticket_has_dual_ids_and_composite_unique() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping ticket_has_dual_ids_and_composite_unique");
        return;
    };

    // Both identifier columns are present.
    let cols: Vec<String> = sqlx::query("SELECT column_name FROM information_schema.columns WHERE table_name='ticket'")
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.get::<String, _>("column_name"))
        .collect();
    assert!(cols.iter().any(|c| c == "ticket_id"), "ticket_id PK column");
    assert!(cols.iter().any(|c| c == "ticketID"), "external ticketID column");

    // Enforcement: duplicate (ticketID, email) is rejected, inside a rolled-back
    // transaction so the test leaves no rows behind.
    let mut tx = pool.begin().await.unwrap();
    let dept_id: i32 =
        sqlx::query_scalar("INSERT INTO department (dept_name) VALUES ($1) RETURNING dept_id")
            .bind(format!("test-dept-{}", uuid_like()))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    sqlx::query(r#"INSERT INTO ticket ("ticketID", dept_id, email) VALUES (654321, $1, 'dup@test.com')"#)
        .bind(dept_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    let dup = sqlx::query(r#"INSERT INTO ticket ("ticketID", dept_id, email) VALUES (654321, $1, 'dup@test.com')"#)
        .bind(dept_id)
        .execute(&mut *tx)
        .await;
    assert!(dup.is_err(), "duplicate (ticketID, email) must be rejected");
    tx.rollback().await.unwrap();
}

/// AC-5: enum-valued columns are text + CHECK; no native PG enum types exist.
#[tokio::test]
async fn enum_columns_are_text_with_checks_no_native_enum() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping enum_columns_are_text_with_checks_no_native_enum");
        return;
    };

    let enum_type_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_type t JOIN pg_namespace n ON n.oid=t.typnamespace \
         WHERE t.typtype='e' AND n.nspname='public'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(enum_type_count, 0, "no native PG enum types may exist");

    // status / source / thread_type are text columns.
    for (table, col) in [
        ("ticket", "status"),
        ("ticket", "source"),
        ("ticket_thread", "thread_type"),
    ] {
        let data_type: String = sqlx::query_scalar(
            "SELECT data_type FROM information_schema.columns \
             WHERE table_name=$1 AND column_name=$2",
        )
        .bind(table)
        .bind(col)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(data_type, "text", "{table}.{col} must be text");
    }

    // A bad status value must be rejected by the CHECK (rolled back).
    let mut tx = pool.begin().await.unwrap();
    let dept_id: i32 =
        sqlx::query_scalar("INSERT INTO department (dept_name) VALUES ($1) RETURNING dept_id")
            .bind(format!("test-dept-{}", uuid_like()))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    let bad = sqlx::query(
        r#"INSERT INTO ticket ("ticketID", dept_id, email, status) VALUES (111111, $1, 'x@test.com', 'bogus')"#,
    )
    .bind(dept_id)
    .execute(&mut *tx)
    .await;
    assert!(bad.is_err(), "status CHECK must reject a non-enum value");
    tx.rollback().await.unwrap();
}

/// A cheap unique-ish suffix so concurrent test runs don't collide on the
/// department unique name (no external crate needed).
fn uuid_like() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
