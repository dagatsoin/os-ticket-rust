//! Integration tests for the TS-M2-A1 attachment schema (migration 0003).
//!
//! DB-backed; skipped (pass with a log line) when `TEST_DATABASE_URL` is unset,
//! so the offline build and a DB-less `cargo test` stay green.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p db --test attachment_schema
//! ```
//!
//! @implements FS-022.12 (D1): attachment_file metadata row (unique content hash).
//! @implements FS-091.4 (ref): ticket_attachment.ref_type M/R/N CHECK + FKs.

use sqlx::postgres::PgPool;

fn test_db_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

async fn migrated_pool() -> Option<PgPool> {
    let url = test_db_url()?;
    let pool = db::connect(&url).await.expect("connect to test DB");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

/// A cheap unique-ish suffix so concurrent runs don't collide.
fn nonce() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

/// The two attachment tables exist after migrating.
#[tokio::test]
async fn attachment_tables_exist() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping attachment_tables_exist");
        return;
    };
    for table in ["attachment_file", "ticket_attachment"] {
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

/// AC-3: a ticket_attachment binds a file to a ticket + thread entry with
/// ref_type M/R/N; FKs are enforced; a fourth ref_type value errors.
#[tokio::test]
async fn ticket_attachment_binds_file_thread_with_checked_ref_type() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping ticket_attachment_binds_file_thread...");
        return;
    };

    // All work happens in a rolled-back transaction → leaves no rows behind.
    let mut tx = pool.begin().await.unwrap();
    let n = nonce();

    let dept_id: i32 =
        sqlx::query_scalar("INSERT INTO department (dept_name) VALUES ($1) RETURNING dept_id")
            .bind(format!("att-dept-{n}"))
            .fetch_one(&mut *tx)
            .await
            .unwrap();

    let ticket_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO ticket ("ticketID", dept_id, email) VALUES ($1, $2, 'a@x.com')
           RETURNING ticket_id"#,
    )
    .bind((n % 900000) as i64 + 100000)
    .bind(dept_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    let thread_id: i64 = sqlx::query_scalar(
        "INSERT INTO ticket_thread (ticket_id, thread_type, body)
         VALUES ($1, 'M', 'hello') RETURNING id",
    )
    .bind(ticket_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    let file_id: i64 = sqlx::query_scalar(
        "INSERT INTO attachment_file (mime, size, hash, name, storage_key)
         VALUES ('application/pdf', 10, $1, 'invoice.pdf', $2) RETURNING id",
    )
    .bind(format!("{:064x}", n))
    .bind(format!("{:02x}/{:02x}/{:064x}", n & 0xff, (n >> 8) & 0xff, n))
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    // Valid binding (ref_type M) succeeds.
    let ok = sqlx::query(
        "INSERT INTO ticket_attachment (ticket_id, file_id, ref_id, ref_type)
         VALUES ($1, $2, $3, 'M')",
    )
    .bind(ticket_id)
    .bind(file_id)
    .bind(thread_id)
    .execute(&mut *tx)
    .await;
    assert!(ok.is_ok(), "valid ticket_attachment insert must succeed");

    tx.rollback().await.unwrap();

    // --- ref_type CHECK: a fourth value (e.g. 'X') is rejected. -----------
    let mut tx = pool.begin().await.unwrap();
    let n = nonce();
    let dept_id: i32 =
        sqlx::query_scalar("INSERT INTO department (dept_name) VALUES ($1) RETURNING dept_id")
            .bind(format!("att-dept-{n}"))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    let ticket_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO ticket ("ticketID", dept_id, email) VALUES ($1, $2, 'b@x.com') RETURNING ticket_id"#,
    )
    .bind((n % 900000) as i64 + 100000)
    .bind(dept_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let thread_id: i64 = sqlx::query_scalar(
        "INSERT INTO ticket_thread (ticket_id, thread_type, body) VALUES ($1, 'M', 'x') RETURNING id",
    )
    .bind(ticket_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let file_id: i64 = sqlx::query_scalar(
        "INSERT INTO attachment_file (hash, name) VALUES ($1, 'f') RETURNING id",
    )
    .bind(format!("{:064x}", n))
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    let bad_type = sqlx::query(
        "INSERT INTO ticket_attachment (ticket_id, file_id, ref_id, ref_type)
         VALUES ($1, $2, $3, 'X')",
    )
    .bind(ticket_id)
    .bind(file_id)
    .bind(thread_id)
    .execute(&mut *tx)
    .await;
    assert!(bad_type.is_err(), "ref_type CHECK must reject a 4th value");
    tx.rollback().await.unwrap();

    // --- orphan file_id FK: an unknown file_id is rejected. ----------------
    let mut tx = pool.begin().await.unwrap();
    let n = nonce();
    let dept_id: i32 =
        sqlx::query_scalar("INSERT INTO department (dept_name) VALUES ($1) RETURNING dept_id")
            .bind(format!("att-dept-{n}"))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    let ticket_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO ticket ("ticketID", dept_id, email) VALUES ($1, $2, 'c@x.com') RETURNING ticket_id"#,
    )
    .bind((n % 900000) as i64 + 100000)
    .bind(dept_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let thread_id: i64 = sqlx::query_scalar(
        "INSERT INTO ticket_thread (ticket_id, thread_type, body) VALUES ($1, 'M', 'x') RETURNING id",
    )
    .bind(ticket_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let orphan = sqlx::query(
        "INSERT INTO ticket_attachment (ticket_id, file_id, ref_id, ref_type)
         VALUES ($1, 999999999, $2, 'M')",
    )
    .bind(ticket_id)
    .bind(thread_id)
    .execute(&mut *tx)
    .await;
    assert!(orphan.is_err(), "orphan file_id FK must be rejected");
    tx.rollback().await.unwrap();
}

/// The content hash UNIQUE constraint backs real dedup (D1).
#[tokio::test]
async fn attachment_file_hash_is_unique() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping attachment_file_hash_is_unique");
        return;
    };
    let mut tx = pool.begin().await.unwrap();
    let hash = format!("{:064x}", nonce());
    sqlx::query("INSERT INTO attachment_file (hash, name) VALUES ($1, 'a')")
        .bind(&hash)
        .execute(&mut *tx)
        .await
        .unwrap();
    let dup = sqlx::query("INSERT INTO attachment_file (hash, name) VALUES ($1, 'b')")
        .bind(&hash)
        .execute(&mut *tx)
        .await;
    assert!(dup.is_err(), "duplicate content hash must be rejected (dedup)");
    tx.rollback().await.unwrap();
}
