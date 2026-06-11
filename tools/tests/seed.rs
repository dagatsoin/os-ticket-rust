//! Integration tests for the TS-M1-A3 seed fixtures.
//!
//! Run against a live Postgres test DB; skipped (pass, with a log line) when
//! `TEST_DATABASE_URL` is unset so offline / DB-less CI stays green:
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p tools --test seed
//! ```
//!
//! @implements BS-091: idempotent seed (TS-M1-A3 AC-1..AC-5).

use sqlx::postgres::PgPool;
use tools::{CFG_DEFAULT_PRIORITY, CFG_DEFAULT_STATUS, PRIORITY_NORMAL, STAFF_PASSWORD, STATUS_OPEN};

fn test_db_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

/// Connect, migrate, and wipe the seedable tables so counts are deterministic.
async fn fresh_pool() -> Option<PgPool> {
    let url = test_db_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    // Clean only the seed-target tables (FK order). The migration test uses
    // rolled-back transactions, so it leaves nothing; this keeps us isolated.
    for stmt in [
        "DELETE FROM ticket_thread",
        "DELETE FROM ticket",
        "DELETE FROM staff",
        "DELETE FROM group_dept_access",
        "DELETE FROM groups",
        "DELETE FROM department",
        "DELETE FROM config WHERE key IN ('default_ticket_status','default_priority')",
    ] {
        sqlx::query(stmt).execute(&pool).await.expect(stmt);
    }
    Some(pool)
}

async fn count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

/// AC-1 + AC-4: one dept/group/staff after seeding, and re-running is a no-op
/// (still exactly 1/1/1, no error).
#[tokio::test]
async fn seed_is_idempotent_with_single_rows() {
    let Some(pool) = fresh_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping seed_is_idempotent_with_single_rows");
        return;
    };

    tools::seed(&pool).await.expect("first seed");
    assert_eq!(count(&pool, "department").await, 1);
    assert_eq!(count(&pool, "groups").await, 1);
    assert_eq!(count(&pool, "staff").await, 1);

    // AC-4: second run produces no duplicates and no error.
    tools::seed(&pool).await.expect("second seed (idempotent)");
    assert_eq!(count(&pool, "department").await, 1, "dept still 1");
    assert_eq!(count(&pool, "groups").await, 1, "group still 1");
    assert_eq!(count(&pool, "staff").await, 1, "staff still 1");

    // The single staff row is the documented `agent` username.
    let username: String = sqlx::query_scalar("SELECT username FROM staff")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(username, "agent");
}

/// AC-2: the seeded argon2id hash verifies for the correct plaintext only.
#[tokio::test]
async fn seeded_hash_verifies_correct_plaintext_only() {
    let Some(pool) = fresh_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping seeded_hash_verifies_correct_plaintext_only");
        return;
    };
    tools::seed(&pool).await.expect("seed");

    let hash: String = sqlx::query_scalar("SELECT passwd FROM staff WHERE username = 'agent'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(hash.starts_with("$argon2id$"), "argon2id hash: {hash}");
    assert!(ost_core::verify_password(STAFF_PASSWORD, &hash).unwrap());
    assert!(!ost_core::verify_password("wrong", &hash).unwrap());
}

/// AC-3: the group grants both M1 flags + access to the seeded department.
#[tokio::test]
async fn seeded_group_grants_flags_and_dept_access() {
    let Some(pool) = fresh_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping seeded_group_grants_flags_and_dept_access");
        return;
    };
    tools::seed(&pool).await.expect("seed");

    let (can_create, can_reply): (bool, bool) =
        sqlx::query_as("SELECT can_create_tickets, can_post_reply FROM groups")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(can_create, "can_create_tickets must be true");
    assert!(can_reply, "can_post_reply must be true");

    // Exactly one group→dept access row, joining the seeded group + Support dept.
    let access: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM group_dept_access gda
           JOIN department d ON d.dept_id = gda.dept_id
          WHERE d.dept_name = 'Support'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(access, 1, "group must have access to the Support dept");
}

/// AC-5: the seeded reference defaults use the FS-091 literals open / normal.
#[tokio::test]
async fn seeded_reference_uses_fs091_literals() {
    let Some(pool) = fresh_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping seeded_reference_uses_fs091_literals");
        return;
    };
    tools::seed(&pool).await.expect("seed");

    let status: String = sqlx::query_scalar("SELECT value FROM config WHERE key = $1")
        .bind(CFG_DEFAULT_STATUS)
        .fetch_one(&pool)
        .await
        .unwrap();
    let priority: String = sqlx::query_scalar("SELECT value FROM config WHERE key = $1")
        .bind(CFG_DEFAULT_PRIORITY)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, STATUS_OPEN, "status literal must be `open`");
    assert_eq!(priority, PRIORITY_NORMAL, "priority literal must be `normal`");
}
