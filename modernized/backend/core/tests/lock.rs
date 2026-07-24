//! Integration tests for the collaborative ticket edit-lock core (US-M3-I2 /
//! FS-021.18, FS-021.20, BS-021.3). Run against a live Postgres test DB; skipped
//! (pass, log line) when `TEST_DATABASE_URL` is unset so DB-less CI stays green:
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p ost_core --test lock
//! ```
//!
//! @implements FS-021.18: acquire/renew is idempotent for the same staff.
//! @implements FS-021.20: "locked by another staff" carries the holder's name.
//! @implements BS-021.3: one active lock per ticket; blocks conflicting replies.

use ost_core::ticket::{create_ticket, NewTicket, NewTicketInput, Ticket};
use ost_core::{acquire_lock, check_lock, get_lock_info, LockError};
use sqlx::postgres::PgPool;

fn test_db_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

/// Connect + migrate + ensure a "Support" dept exists (create core uses the
/// lowest-id dept as default). Returns the pool + default dept id.
async fn setup() -> Option<(PgPool, i32)> {
    let url = test_db_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let dept_id: i32 = sqlx::query_scalar(
        "INSERT INTO department (dept_name) VALUES ('Support') \
         ON CONFLICT (dept_name) DO UPDATE SET updated = now() RETURNING dept_id",
    )
    .fetch_one(&pool)
    .await
    .expect("ensure Support dept");
    Some((pool, dept_id))
}

/// Insert a staff row directly (unique username per test run), returning its id.
async fn make_staff(
    pool: &PgPool,
    dept_id: i32,
    username: &str,
    firstname: &str,
    lastname: &str,
) -> i32 {
    // A throwaway enabled group so the FK is satisfied.
    let group_id: i32 = sqlx::query_scalar(
        "INSERT INTO groups (group_name, group_enabled) VALUES ($1, true) RETURNING group_id",
    )
    .bind(format!("lock-test-{username}"))
    .fetch_one(pool)
    .await
    .expect("group");

    sqlx::query_scalar(
        "INSERT INTO staff (group_id, dept_id, username, firstname, lastname, passwd, email, isactive)
         VALUES ($1, $2, $3, $4, $5, 'x', $6, true)
         RETURNING staff_id",
    )
    .bind(group_id)
    .bind(dept_id)
    .bind(username)
    .bind(firstname)
    .bind(lastname)
    .bind(format!("{username}@example.com"))
    .fetch_one(pool)
    .await
    .expect("staff")
}

async fn make_ticket(pool: &PgPool) -> Ticket {
    let input = NewTicket::validated(NewTicketInput {
        email: "requester@example.com".into(),
        name: "Reqq Ester".into(),
        subject: "Lock subject".into(),
        body: "Body of the lock test ticket.".into(),
        ..Default::default()
    })
    .expect("valid input");
    create_ticket(pool, &input).await.expect("create ticket")
}

async fn cleanup(pool: &PgPool, ticket_id: i64, staff_ids: &[i32]) {
    sqlx::query("DELETE FROM ticket_lock WHERE ticket_id = $1")
        .bind(ticket_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM ticket_thread WHERE ticket_id = $1")
        .bind(ticket_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM ticket WHERE ticket_id = $1")
        .bind(ticket_id)
        .execute(pool)
        .await
        .unwrap();
    for &s in staff_ids {
        sqlx::query("DELETE FROM staff WHERE staff_id = $1")
            .bind(s)
            .execute(pool)
            .await
            .unwrap();
    }
}

fn uniq(prefix: &str) -> String {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{n}")
}

// --- DEFECT 1: same-staff re-acquire is idempotent (renews, never fails) -----

/// @implements FS-021.18: re-acquiring an own lock renews it and never errors.
#[tokio::test]
async fn same_staff_reacquire_is_idempotent() {
    let Some((pool, dept)) = setup().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping same_staff_reacquire_is_idempotent");
        return;
    };
    let a = make_staff(&pool, dept, &uniq("agentA"), "Agent", "One").await;
    let t = make_ticket(&pool).await;

    let first = acquire_lock(&pool, t.ticket_id, a, 2).await.expect("first acquire");
    // Re-acquire twice more for the SAME staff — each must succeed (renew) and
    // return the SAME lock id (no spurious failure, no duplicate row).
    let second = acquire_lock(&pool, t.ticket_id, a, 2).await.expect("second acquire (renew)");
    let third = acquire_lock(&pool, t.ticket_id, a, 2).await.expect("third acquire (renew)");
    assert_eq!(first.lock_id, second.lock_id, "renew keeps the same lock id");
    assert_eq!(second.lock_id, third.lock_id, "renew keeps the same lock id");

    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ticket_lock WHERE ticket_id = $1")
        .bind(t.ticket_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 1, "exactly one lock row for the ticket");

    cleanup(&pool, t.ticket_id, &[a]).await;
}

/// Concurrent same-staff acquires (React double-invoke firing the lock POST
/// twice) must both succeed — the ON CONFLICT/INSERT-IGNORE path never 500s.
///
/// @implements FS-021.18: race-safe acquire (legacy INSERT IGNORE).
#[tokio::test]
async fn concurrent_same_staff_acquire_never_errors() {
    let Some((pool, dept)) = setup().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping concurrent_same_staff_acquire_never_errors");
        return;
    };
    let a = make_staff(&pool, dept, &uniq("agentC"), "Agent", "Cee").await;
    let t = make_ticket(&pool).await;

    let (p1, p2) = (pool.clone(), pool.clone());
    let (tid, sid) = (t.ticket_id, a);
    let h1 = tokio::spawn(async move { acquire_lock(&p1, tid, sid, 2).await });
    let h2 = tokio::spawn(async move { acquire_lock(&p2, tid, sid, 2).await });
    let r1 = h1.await.unwrap();
    let r2 = h2.await.unwrap();
    assert!(r1.is_ok(), "concurrent acquire #1 ok: {r1:?}");
    assert!(r2.is_ok(), "concurrent acquire #2 ok: {r2:?}");

    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ticket_lock WHERE ticket_id = $1")
        .bind(t.ticket_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 1, "still exactly one lock row after the race");

    cleanup(&pool, t.ticket_id, &[a]).await;
}

// --- DEFECT 2: different-staff acquire → locked_by_other + name --------------

/// @implements FS-021.20: another staff's acquire is denied with the holder name.
#[tokio::test]
async fn different_staff_acquire_is_denied_with_name() {
    let Some((pool, dept)) = setup().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping different_staff_acquire_is_denied_with_name");
        return;
    };
    let a = make_staff(&pool, dept, &uniq("agentA"), "Agent", "One").await;
    let b = make_staff(&pool, dept, &uniq("agentB"), "Agent", "Two").await;
    let t = make_ticket(&pool).await;

    acquire_lock(&pool, t.ticket_id, a, 2).await.expect("A acquires");
    // B tries to acquire the same live lock → denied, carrying A's display name.
    let err = acquire_lock(&pool, t.ticket_id, b, 2).await.expect_err("B denied");
    match err {
        LockError::LockedByOther(name) => assert_eq!(name, "Agent One"),
        other => panic!("expected LockedByOther, got {other:?}"),
    }

    cleanup(&pool, t.ticket_id, &[a, b]).await;
}

/// get_lock_info: locked_by_other=true + name for a non-holder viewer; false with
/// only the expiry for the holder's own view.
///
/// @implements FS-021.18/.20: lock state surfaced to the ticket-detail viewer.
#[tokio::test]
async fn lock_info_reports_other_holder_with_name() {
    let Some((pool, dept)) = setup().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping lock_info_reports_other_holder_with_name");
        return;
    };
    let a = make_staff(&pool, dept, &uniq("agentA"), "Agent", "One").await;
    let b = make_staff(&pool, dept, &uniq("agentB"), "Agent", "Two").await;
    let t = make_ticket(&pool).await;

    acquire_lock(&pool, t.ticket_id, a, 2).await.expect("A acquires");

    // B (non-holder) sees "locked by other" + A's name + an expiry.
    let for_b = get_lock_info(&pool, t.ticket_id, b).await.expect("info for B");
    assert!(for_b.locked_by_other);
    assert_eq!(for_b.locked_by_name.as_deref(), Some("Agent One"));
    assert!(for_b.expires_at.is_some());

    // A (holder) sees its own lock: not "other", no name, but an expiry.
    let for_a = get_lock_info(&pool, t.ticket_id, a).await.expect("info for A");
    assert!(!for_a.locked_by_other);
    assert!(for_a.locked_by_name.is_none());
    assert!(for_a.expires_at.is_some());

    cleanup(&pool, t.ticket_id, &[a, b]).await;
}

// --- DEFECT 3: reply blocked when locked by other (check_lock) ---------------

/// The reply route gates on `check_lock`: it returns the *other* holder's name
/// (→ 409 for a non-holder) and None for the holder (→ reply allowed).
///
/// @implements BS-021.3: lock blocks conflicting replies.
#[tokio::test]
async fn check_lock_blocks_other_allows_holder() {
    let Some((pool, dept)) = setup().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping check_lock_blocks_other_allows_holder");
        return;
    };
    let a = make_staff(&pool, dept, &uniq("agentA"), "Agent", "One").await;
    let b = make_staff(&pool, dept, &uniq("agentB"), "Agent", "Two").await;
    let t = make_ticket(&pool).await;

    acquire_lock(&pool, t.ticket_id, a, 2).await.expect("A acquires");

    // B is blocked (holder name surfaced); A (the holder) is allowed.
    let blocked = check_lock(&pool, t.ticket_id, b).await.expect("check for B");
    assert_eq!(blocked.as_deref(), Some("Agent One"));
    let allowed = check_lock(&pool, t.ticket_id, a).await.expect("check for A");
    assert!(allowed.is_none(), "holder is not blocked");

    cleanup(&pool, t.ticket_id, &[a, b]).await;
}

// --- expiry allows re-acquire by a different staff --------------------------

/// An expired lock is cleared on acquire, so another staff can then take it.
///
/// @implements FS-021.18: expired locks cleared on acquire (legacy DELETE ... < NOW()).
#[tokio::test]
async fn expired_lock_allows_reacquire_by_other() {
    let Some((pool, dept)) = setup().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping expired_lock_allows_reacquire_by_other");
        return;
    };
    let a = make_staff(&pool, dept, &uniq("agentA"), "Agent", "One").await;
    let b = make_staff(&pool, dept, &uniq("agentB"), "Agent", "Two").await;
    let t = make_ticket(&pool).await;

    // Seed an already-expired lock held by A (expire in the past).
    sqlx::query(
        "INSERT INTO ticket_lock (ticket_id, staff_id, expire) VALUES ($1, $2, now() - interval '1 minute')",
    )
    .bind(t.ticket_id)
    .bind(a)
    .execute(&pool)
    .await
    .expect("seed expired lock");

    // get_lock_info for B reads it as unlocked (expired cleared).
    let info = get_lock_info(&pool, t.ticket_id, b).await.expect("info");
    assert!(!info.locked_by_other);
    assert!(info.expires_at.is_none());

    // B can now acquire the lock (A's expired one was cleared).
    let res = acquire_lock(&pool, t.ticket_id, b, 2).await.expect("B re-acquires");
    assert!(res.remaining_seconds > 0);

    // And A is now the blocked one.
    let blocked = check_lock(&pool, t.ticket_id, a).await.expect("check for A");
    assert_eq!(blocked.as_deref(), Some("Agent Two"));

    cleanup(&pool, t.ticket_id, &[a, b]).await;
}
