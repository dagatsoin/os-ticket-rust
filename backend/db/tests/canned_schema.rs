//! Integration tests for the TS-M2-D1 canned-response schema (migration 0004).
//!
//! DB-backed; skipped (pass with a log line) when `TEST_DATABASE_URL` is unset,
//! so the offline build and a DB-less `cargo test` stay green.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p db --test canned_schema
//! ```
//!
//! @implements FS-022.14: canned_response (title-unique, dept scope, enabled flag).
//! @implements BS-022.1 / BS-022.2: dept scope (0 = all) + enabled flag persist.

use sqlx::postgres::PgPool;
use sqlx::Row;

fn test_db_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

async fn migrated_pool() -> Option<PgPool> {
    let url = test_db_url()?;
    let pool = db::connect(&url).await.expect("connect to test DB");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

fn nonce() -> u128 {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static C: AtomicU64 = AtomicU64::new(0);
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    t.wrapping_add(C.fetch_add(1, Ordering::Relaxed) as u128)
}

/// The two canned tables exist after migrating.
#[tokio::test]
async fn canned_tables_exist() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping canned_tables_exist");
        return;
    };
    for table in ["canned_response", "canned_attachment"] {
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
}

/// AC-1: a canned_response with title uniqueness + dept scope + enabled flag
/// persists; a third insert reusing a title errors (unique constraint);
/// `dept_id=0` and `isenabled` persist as written.
#[tokio::test]
async fn canned_response_title_unique_dept_scope_enabled_persist() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping canned_response_title_unique...");
        return;
    };

    // Rolled-back transaction → leaves no rows behind.
    let mut tx = pool.begin().await.unwrap();
    let n = nonce();
    let title_a = format!("Canned A {n}");
    let title_b = format!("Canned B {n}");

    // Insert two responses: one dept-0 enabled, one dept-scoped disabled.
    let row_a = sqlx::query(
        "INSERT INTO canned_response (title, response, dept_id, isenabled)
         VALUES ($1, 'body a %{ticket.number}', 0, true)
         RETURNING canned_id, dept_id, isenabled, response",
    )
    .bind(&title_a)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let dept_a: i32 = row_a.get("dept_id");
    let enabled_a: bool = row_a.get("isenabled");
    let body_a: String = row_a.get("response");
    assert_eq!(dept_a, 0, "dept_id=0 (all departments) persists as written");
    assert!(enabled_a, "isenabled=true persists as written");
    assert!(body_a.contains("%{ticket.number}"), "body persists verbatim");

    let row_b = sqlx::query(
        "INSERT INTO canned_response (title, response, dept_id, isenabled)
         VALUES ($1, 'body b', 7, false)
         RETURNING dept_id, isenabled",
    )
    .bind(&title_b)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let dept_b: i32 = row_b.get("dept_id");
    let enabled_b: bool = row_b.get("isenabled");
    assert_eq!(dept_b, 7, "a non-zero dept scope persists");
    assert!(!enabled_b, "isenabled=false persists as written");

    // A third insert reusing title_a errors on the UNIQUE constraint.
    let dup = sqlx::query("INSERT INTO canned_response (title) VALUES ($1)")
        .bind(&title_a)
        .execute(&mut *tx)
        .await;
    assert!(dup.is_err(), "duplicate title must be rejected (unique)");

    tx.rollback().await.unwrap();
}

/// A canned_attachment binds a canned_response to an attachment_file; the FK is
/// enforced; a file binds to a canned response at most once (unique pair).
#[tokio::test]
async fn canned_attachment_binds_file_with_unique_pair() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping canned_attachment_binds_file...");
        return;
    };
    let mut tx = pool.begin().await.unwrap();
    let n = nonce();

    let canned_id: i32 = sqlx::query_scalar(
        "INSERT INTO canned_response (title) VALUES ($1) RETURNING canned_id",
    )
    .bind(format!("Canned C {n}"))
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    let file_id: i64 = sqlx::query_scalar(
        "INSERT INTO attachment_file (hash, name) VALUES ($1, 'policy.txt') RETURNING id",
    )
    .bind(format!("{n:064x}"))
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    // First binding succeeds.
    let ok = sqlx::query(
        "INSERT INTO canned_attachment (canned_id, file_id) VALUES ($1, $2)",
    )
    .bind(canned_id)
    .bind(file_id)
    .execute(&mut *tx)
    .await;
    assert!(ok.is_ok(), "valid canned_attachment binding must succeed");

    // Re-binding the same (canned_id, file_id) errors on the unique pair.
    let dup = sqlx::query(
        "INSERT INTO canned_attachment (canned_id, file_id) VALUES ($1, $2)",
    )
    .bind(canned_id)
    .bind(file_id)
    .execute(&mut *tx)
    .await;
    assert!(dup.is_err(), "duplicate (canned_id, file_id) must be rejected");

    // An unknown file_id is rejected by the FK.
    let orphan = sqlx::query(
        "INSERT INTO canned_attachment (canned_id, file_id) VALUES ($1, 999999999)",
    )
    .bind(canned_id)
    .execute(&mut *tx)
    .await;
    assert!(orphan.is_err(), "orphan file_id FK must be rejected");

    tx.rollback().await.unwrap();
}
