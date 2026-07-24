//! Integration tests for the TS-M2-prep `--reset` purge (truncation half).
//!
//! DB-backed; skipped (pass with a log line) when `TEST_DATABASE_URL` is unset.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p tools --test reset
//! ```
//!
//! @implements TS-M2-prep: `--reset` (AC-1 purge, AC-2 preserve baseline,
//!   AC-3 orphan-blob reclamation preserving the seeded canned blob, AC-4 plain
//!   seed non-destructive) + the production-safety dev-DB guard.

use ost_core::attachment::{insert_attachment, AttachmentSpec};
use ost_core::BlobStore;
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

async fn ticket_exists(pool: &PgPool, ticket_id: i64) -> bool {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ticket WHERE ticket_id = $1)")
        .bind(ticket_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn threads_for(pool: &PgPool, ticket_id: i64) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM ticket_thread WHERE ticket_id = $1")
        .bind(ticket_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn attachments_for(pool: &PgPool, ticket_id: i64) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM ticket_attachment WHERE ticket_id = $1")
        .bind(ticket_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Count rows of `table` whose `col` equals `val` (used to assert a seeded
/// identity stays a single row independent of other crates' shared-DB rows).
async fn rows_named(pool: &PgPool, table: &str, col: &str, val: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table} WHERE {col} = $1"))
        .bind(val)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Suite-unique advisory-lock key for the reset tests.
const RESET_LOCK_KEY: i64 = 0x05_1C_5E_72_E5_E7;

/// Hold a session-level advisory lock so the ticket-mutating reset tests
/// serialize against each other on the shared DB (cargo runs a binary's tests
/// concurrently). The held connection is returned; the lock releases when the
/// connection is dropped back to the pool at end of the test. Only these tests
/// take this key, which suffices because within `cargo test -p tools` they are
/// the only ticket inserters (the M1 seed.rs tests never insert tickets).
async fn acquire_reset_lock(pool: &PgPool) -> sqlx::pool::PoolConnection<sqlx::Postgres> {
    let mut conn = pool.acquire().await.expect("acquire conn for advisory lock");
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(RESET_LOCK_KEY)
        .execute(&mut *conn)
        .await
        .expect("take advisory lock");
    conn
}

/// A unique-ish nonce so reruns and concurrent tests never collide on the
/// `ticketID`/content-hash unique constraints (the reset's deferred blob half
/// leaves `attachment_file` rows behind, so fixed hashes would clash on rerun).
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

/// Create a ticket with a thread entry + a bound attachment, so reset has
/// ticket-scoped rows across all three tables to purge. Returns the ticket_id.
async fn make_ticket_with_attachment(pool: &PgPool) -> i64 {
    let n = nonce();
    let dept_id: i32 = sqlx::query_scalar("SELECT dept_id FROM department LIMIT 1")
        .fetch_one(pool)
        .await
        .expect("seeded department must exist");
    let ticket_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO ticket ("ticketID", dept_id, email) VALUES ($1, $2, $3)
           RETURNING ticket_id"#,
    )
    .bind((n % 900000) as i64 + 100000)
    .bind(dept_id)
    .bind(format!("r-{n}@x.com"))
    .fetch_one(pool)
    .await
    .unwrap();
    let thread_id: i64 = sqlx::query_scalar(
        "INSERT INTO ticket_thread (ticket_id, thread_type, body) VALUES ($1, 'M', 'hi') RETURNING id",
    )
    .bind(ticket_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let file_id: i64 = sqlx::query_scalar(
        "INSERT INTO attachment_file (hash, name) VALUES ($1, 'r.pdf') RETURNING id",
    )
    .bind(format!("{n:064x}"))
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ticket_attachment (ticket_id, file_id, ref_id, ref_type)
         VALUES ($1, $2, $3, 'M')",
    )
    .bind(ticket_id)
    .bind(file_id)
    .bind(thread_id)
    .execute(pool)
    .await
    .unwrap();
    ticket_id
}

/// AC-1: `--reset` purges all tickets + thread entries + ticket attachments.
/// AC-2: it preserves the seeded department/group/staff baseline.
#[tokio::test]
async fn reset_purges_tickets_and_preserves_baseline() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping reset_purges_tickets_and_preserves_baseline");
        return;
    };
    // Serialize against the other ticket-mutating reset test on the shared DB.
    let _lock = acquire_reset_lock(&pool).await;

    // Seed the baseline, then create a couple of tickets-with-attachments.
    tools::seed(&pool).await.expect("seed baseline");
    let t1 = make_ticket_with_attachment(&pool).await;
    let t2 = make_ticket_with_attachment(&pool).await;
    assert!(ticket_exists(&pool, t1).await && ticket_exists(&pool, t2).await);

    // Reset.
    tools::reset(&pool).await.expect("reset");

    // AC-1: every ticket + its thread entries + its attachments are purged.
    // (Asserted on the specific rows we created so a concurrent ticket insert
    // in another test crate cannot make this flaky; the TRUNCATE is global so
    // these necessarily vanish.)
    assert!(!ticket_exists(&pool, t1).await, "ticket {t1} purged");
    assert!(!ticket_exists(&pool, t2).await, "ticket {t2} purged");
    assert_eq!(
        threads_for(&pool, t1).await + threads_for(&pool, t2).await,
        0,
        "thread entries for purged tickets are gone"
    );
    assert_eq!(
        attachments_for(&pool, t1).await + attachments_for(&pool, t2).await,
        0,
        "ticket_attachment bindings for purged tickets are gone"
    );
    // `session` is included in the truncate set (verified structurally below in
    // `session_is_in_the_truncate_set`); we do not assert a global session count
    // here because a concurrent auth test in another crate may create one.

    // AC-2: the seeded baseline survives — asserted on the seeded identities
    // (unique by name/username) rather than global counts, which other test
    // crates contaminate. Each remains exactly one row after the reset+reseed.
    assert_eq!(
        rows_named(&pool, "department", "dept_name", tools::DEPT_NAME).await,
        1,
        "seeded department preserved"
    );
    assert_eq!(
        rows_named(&pool, "groups", "group_name", tools::GROUP_NAME).await,
        1,
        "seeded group preserved"
    );
    // The four M2 config keys (preserved + re-seeded) all resolve.
    for key in [
        "allow_attachments",
        "allowed_filetypes",
        "max_file_size",
        "helpdesk_url",
    ] {
        assert_eq!(rows_named(&pool, "config", "key", key).await, 1, "config `{key}` preserved");
    }

    // The agent account still resolves with its hash (login would succeed).
    let agent_hash: Option<String> =
        sqlx::query_scalar("SELECT passwd FROM staff WHERE username = $1")
            .bind(tools::STAFF_USERNAME)
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert!(agent_hash.is_some(), "seeded agent account preserved");

    // Orphan-blob reclamation (AC-3) is covered by its own test below; here we
    // only assert the canned policy.txt file row survives the reset (it is bound
    // by canned_attachment, so it is never an orphan).
    let canned_files: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM attachment_file af
         WHERE EXISTS (SELECT 1 FROM canned_attachment ca WHERE ca.file_id = af.id)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(canned_files >= 1, "the seeded canned policy.txt file survives");
}

/// AC-4: plain `seed` (no flag) is non-destructive — it never purges tickets.
#[tokio::test]
async fn plain_seed_is_non_destructive() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping plain_seed_is_non_destructive");
        return;
    };
    // Serialize against the purging reset test (it TRUNCATEs all tickets).
    let _lock = acquire_reset_lock(&pool).await;

    tools::seed(&pool).await.expect("seed baseline");
    let t = make_ticket_with_attachment(&pool).await;
    assert!(ticket_exists(&pool, t).await);

    // Plain seed must leave the ticket untouched + reseed baseline idempotently.
    tools::seed(&pool).await.expect("plain re-seed");
    assert!(
        ticket_exists(&pool, t).await,
        "plain seed must not purge tickets (M1 contract)"
    );
    // Idempotency is checked on the seeded identities (unique by name/username),
    // not global counts — other test crates share this DB and add their own rows.
    assert_eq!(
        rows_named(&pool, "staff", "username", tools::STAFF_USERNAME).await,
        1,
        "the seeded agent stays a single row (idempotent)"
    );
    assert_eq!(
        rows_named(&pool, "department", "dept_name", tools::DEPT_NAME).await,
        1,
        "the seeded department stays a single row (idempotent)"
    );

    // Clean up our stray ticket so we don't leak into other tests' counts.
    sqlx::query("DELETE FROM ticket WHERE ticket_id = $1")
        .bind(t)
        .execute(&pool)
        .await
        .unwrap();
}

/// Guard: the dev-DB allow-list contains exactly the two recognised dev DBs.
#[tokio::test]
async fn dev_guard_recognises_test_database() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping dev_guard_recognises_test_database");
        return;
    };
    // Against osticket_test the guard passes and returns the live DB name.
    let name = tools::assert_dev_database(&pool)
        .await
        .expect("osticket_test must be a recognised dev DB");
    assert!(tools::ALLOWED_RESET_DBS.contains(&name.as_str()));
}

/// Guard semantics (no DB needed): a non-dev DB name is refused.
#[test]
fn guard_allow_list_is_exactly_dev_and_test() {
    assert_eq!(tools::ALLOWED_RESET_DBS, ["osticket_dev", "osticket_test"]);
    assert!(!tools::ALLOWED_RESET_DBS.contains(&"osticket_prod"));
    assert!(!tools::ALLOWED_RESET_DBS.contains(&"postgres"));
}

/// Structural: the truncate set purges exactly the ticket-scoped tables +
/// sessions, and (by omission) preserves the baseline + canned + M3 tables.
#[test]
fn session_is_in_the_truncate_set() {
    let set = tools::RESET_TRUNCATE_TABLES;
    for purged in ["ticket_attachment", "ticket_thread", "ticket", "session"] {
        assert!(set.contains(&purged), "`{purged}` must be truncated");
    }
    // Baseline + future canned + M3 tables must NOT be in the purge set.
    for preserved in [
        "department",
        "groups",
        "group_dept_access",
        "staff",
        "sla",
        "config",
        "attachment_file", // pruned selectively (orphans only), never TRUNCATEd
        "canned_response", // seeded canned samples — preserved by omission
        "canned_attachment",
        "team",        // M3: seeded team — preserved by omission
        "team_member", // M3: seeded team membership — preserved by omission
        "help_topic",  // M3: seeded help topics — preserved by omission
    ] {
        assert!(
            !set.contains(&preserved),
            "`{preserved}` must be preserved (not truncated)"
        );
    }
}

/// The blob store resolved the same way the reclamation + seed do (from
/// `BLOB_ROOT`, default `<cwd>/var/blobs`), so the test agrees on locations.
fn env_store() -> BlobStore {
    BlobStore::from_env(std::env::current_dir().unwrap_or_else(|_| ".".into()))
}

/// Create a ticket whose `M` entry binds a freshly-stored unique blob (a real
/// on-disk file + attachment_file row), so the reset has a ticket-only blob to
/// reclaim. Returns (ticket_id, the blob hash).
async fn make_ticket_with_stored_blob(pool: &PgPool, store: &BlobStore) -> (i64, String) {
    let n = nonce();
    let bytes = format!("ticket-only-blob-{n}").into_bytes();
    let spec = AttachmentSpec::new(format!("doc-{n}.pdf"), "application/pdf", bytes.clone());
    let hash = store.put(&bytes).await.unwrap();

    let dept_id: i32 = sqlx::query_scalar("SELECT dept_id FROM department LIMIT 1")
        .fetch_one(pool)
        .await
        .unwrap();
    let ticket_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO ticket ("ticketID", dept_id, email) VALUES ($1, $2, $3)
           RETURNING ticket_id"#,
    )
    .bind((n % 900000) as i64 + 100000)
    .bind(dept_id)
    .bind(format!("blob-{n}@x.com"))
    .fetch_one(pool)
    .await
    .unwrap();
    let thread_id: i64 = sqlx::query_scalar(
        "INSERT INTO ticket_thread (ticket_id, thread_type, body) VALUES ($1, 'M', 'hi') RETURNING id",
    )
    .bind(ticket_id)
    .fetch_one(pool)
    .await
    .unwrap();

    let mut tx = pool.begin().await.unwrap();
    insert_attachment(&mut tx, ticket_id, thread_id, "M", &spec)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    (ticket_id, hash)
}

/// AC-3: `--reset` reclaims orphaned blobs (ticket-only) but keeps referenced
/// ones (the seeded canned policy.txt, bound by canned_attachment).
#[tokio::test]
async fn reset_reclaims_orphan_blobs_keeps_canned() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping reset_reclaims_orphan_blobs_keeps_canned");
        return;
    };
    // Serialize against the other ticket-mutating reset tests (they TRUNCATE).
    let _lock = acquire_reset_lock(&pool).await;

    let store = env_store();
    // Seed first (so the canned policy.txt blob + binding exist).
    tools::seed(&pool).await.expect("seed baseline + canned");

    // The canned policy.txt blob: capture its hash (still referenced after reset).
    let canned_hash: String = sqlx::query_scalar(
        "SELECT af.hash FROM attachment_file af
         JOIN canned_attachment ca ON ca.file_id = af.id
         JOIN canned_response cr ON cr.canned_id = ca.canned_id
         WHERE cr.title = $1
         LIMIT 1",
    )
    .bind(tools::CANNED_TITLE_POLICY)
    .fetch_one(&pool)
    .await
    .expect("seeded canned blob must exist");
    assert!(
        store.exists(&canned_hash).await.unwrap(),
        "canned policy.txt blob is on disk before reset"
    );

    // Create a ticket-with-stored-blob → a ticket-only (orphan-after-purge) blob.
    let (ticket_id, ticket_hash) = make_ticket_with_stored_blob(&pool, &store).await;
    assert!(
        store.exists(&ticket_hash).await.unwrap(),
        "ticket-only blob is on disk before reset"
    );
    // (Distinct content ⇒ distinct hashes ⇒ no shared-blob confound.)
    assert_ne!(ticket_hash, canned_hash);

    // Reset: truncates the ticket bindings, re-seeds canned, then reclaims.
    tools::reset(&pool).await.expect("reset");

    // The ticket-only blob row + on-disk file are reclaimed.
    assert!(!ticket_exists(&pool, ticket_id).await, "ticket purged");
    let ticket_file_rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM attachment_file WHERE hash = $1")
            .bind(&ticket_hash)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(ticket_file_rows, 0, "orphan attachment_file row reclaimed");
    assert!(
        !store.exists(&ticket_hash).await.unwrap(),
        "orphan blob removed from disk"
    );

    // The canned policy.txt blob row + on-disk file survive (still referenced).
    let canned_file_rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM attachment_file WHERE hash = $1")
            .bind(&canned_hash)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(canned_file_rows, 1, "canned attachment_file row preserved");
    assert!(
        store.exists(&canned_hash).await.unwrap(),
        "seeded canned policy.txt blob preserved on disk"
    );
}
