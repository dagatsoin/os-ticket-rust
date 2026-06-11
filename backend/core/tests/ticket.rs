//! Integration tests for the TS-M1-B1 ticket service core. Run against a live
//! Postgres test DB (schema + seed applied); skipped (pass, with a log line)
//! when `TEST_DATABASE_URL` is unset so offline / DB-less CI stays green:
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p ost_core --test ticket
//! ```
//!
//! @implements BS-011: atomic create (ticket + first M entry) — AC-1.
//! @implements BS-021: append R entry, ordered thread — AC-2.
//! @implements FS-091.2: distinct numbers + collision recovery via retry — AC-3.

use std::cell::Cell;

use ost_core::ticket::{
    append_thread_entry, create_ticket, create_ticket_with_numbers, load_thread, NewThreadEntry,
    NewTicket, NewTicketInput, Ticket, TICKET_NUMBER_MAX, TICKET_NUMBER_MIN,
};
use sqlx::postgres::PgPool;

fn test_db_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

/// Connect + migrate + ensure a default department exists (the seeded "Support"
/// dept is created by the seed task; we create it directly to avoid a dep cycle
/// on `tools`, which depends on `ost_core`).
async fn pool_with_default_dept() -> Option<(PgPool, i32)> {
    let url = test_db_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    // Idempotently ensure a "Support" department (the create core uses the
    // lowest-id department as the default).
    let dept_id: i32 = sqlx::query_scalar(
        "INSERT INTO department (dept_name) VALUES ('Support') \
         ON CONFLICT (dept_name) DO UPDATE SET updated = now() RETURNING dept_id",
    )
    .fetch_one(&pool)
    .await
    .expect("ensure Support dept");
    Some((pool, dept_id))
}

fn sample_input() -> NewTicketInput {
    NewTicketInput {
        email: "requester@example.com".into(),
        name: "Reqq Ester".into(),
        subject: "Cannot log in".into(),
        body: "I forgot my password and the reset link is broken.".into(),
        ..Default::default()
    }
}

async fn cleanup(pool: &PgPool, ticket: &Ticket) {
    sqlx::query("DELETE FROM ticket_thread WHERE ticket_id = $1")
        .bind(ticket.ticket_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM ticket WHERE ticket_id = $1")
        .bind(ticket.ticket_id)
        .execute(pool)
        .await
        .unwrap();
}

// --- AC-1: atomic create + first M entry -----------------------------------

#[tokio::test]
async fn create_ticket_persists_ticket_with_first_m_entry() {
    let Some((pool, default_dept)) = pool_with_default_dept().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping create_ticket_persists_ticket_with_first_m_entry");
        return;
    };
    let input = NewTicket::validated(sample_input()).expect("valid input");
    let ticket = create_ticket(&pool, &input).await.expect("create");

    // 6-digit number, seeded dept, open status.
    assert!((TICKET_NUMBER_MIN..=TICKET_NUMBER_MAX).contains(&ticket.ticket_number));
    assert_eq!(ticket.dept_id, default_dept, "defaults to the seeded dept");
    assert_eq!(ticket.status, "open");

    // Exactly one thread entry, type M, holding the body.
    let thread = load_thread(&pool, ticket.ticket_id).await.unwrap();
    assert_eq!(thread.len(), 1, "exactly one entry on creation");
    assert_eq!(thread[0].thread_type, "M");
    assert!(thread[0].body.contains("forgot my password"));

    cleanup(&pool, &ticket).await;
}

/// AC-1 atomicity: a forced failure (a non-existent explicit department ⇒ FK
/// violation on the ticket INSERT) leaves NO partial ticket row.
#[tokio::test]
async fn create_ticket_is_atomic_no_partial_row_on_failure() {
    let Some((pool, _)) = pool_with_default_dept().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping create_ticket_is_atomic_no_partial_row_on_failure");
        return;
    };

    let marker_email = format!(
        "atomic-{}@example.com",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM ticket WHERE email = $1")
        .bind(&marker_email)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, 0);

    let mut input = NewTicket::validated(NewTicketInput {
        email: marker_email.clone(),
        name: "Atom".into(),
        subject: "boom".into(),
        body: "body".into(),
        ..Default::default()
    })
    .unwrap();
    // Force the ticket INSERT to fail (no such department → FK violation).
    input.dept_id = Some(-9999);

    let result = create_ticket(&pool, &input).await;
    assert!(result.is_err(), "create must fail on a bad department");

    // No partial ticket row was committed.
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM ticket WHERE email = $1")
        .bind(&marker_email)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(after, 0, "a forced failure must leave NO partial ticket row");
}

// --- AC-2: append R entry, ordered thread ----------------------------------

#[tokio::test]
async fn append_thread_entry_adds_ordered_response() {
    let Some((pool, _)) = pool_with_default_dept().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping append_thread_entry_adds_ordered_response");
        return;
    };
    let input = NewTicket::validated(sample_input()).unwrap();
    let ticket = create_ticket(&pool, &input).await.unwrap();

    let reply = NewThreadEntry::response("Agent One", None, "Here is how to reset it.");
    let entry = append_thread_entry(&pool, ticket.ticket_id, &reply)
        .await
        .expect("append R");
    assert_eq!(entry.thread_type, "R");

    // The thread returns M then R in chronological order.
    let thread = load_thread(&pool, ticket.ticket_id).await.unwrap();
    let types: Vec<&str> = thread.iter().map(|e| e.thread_type.as_str()).collect();
    assert_eq!(types, vec!["M", "R"], "M precedes R chronologically");

    cleanup(&pool, &ticket).await;
}

#[tokio::test]
async fn append_thread_entry_rejects_unknown_ticket() {
    let Some((pool, _)) = pool_with_default_dept().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping append_thread_entry_rejects_unknown_ticket");
        return;
    };
    let reply = NewThreadEntry::response("Agent", None, "x");
    let err = append_thread_entry(&pool, -1, &reply).await;
    assert!(err.is_err(), "appending to a missing ticket is an error");
}

// --- AC-3: distinct numbers + collision recovery ---------------------------

#[tokio::test]
async fn ticket_numbers_are_distinct_across_creates() {
    let Some((pool, _)) = pool_with_default_dept().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping ticket_numbers_are_distinct_across_creates");
        return;
    };
    let input = NewTicket::validated(sample_input()).unwrap();

    let mut created = Vec::new();
    for _ in 0..8 {
        created.push(create_ticket(&pool, &input).await.expect("create"));
    }
    let mut numbers: Vec<i64> = created.iter().map(|t| t.ticket_number).collect();
    numbers.sort_unstable();
    let distinct = {
        let mut n = numbers.clone();
        n.dedup();
        n.len()
    };
    assert_eq!(distinct, numbers.len(), "all ticket numbers distinct");

    for t in &created {
        cleanup(&pool, t).await;
    }
}

/// AC-3: a forced number collision recovers via retry-on-conflict — the first
/// INSERT hits the UNIQUE(ticketID) constraint and the allocator re-rolls and
/// retries the INSERT (NOT a SELECT-then-INSERT). A deterministic number
/// generator feeds the already-taken number first, then a fresh one.
#[tokio::test]
async fn ticket_number_collision_recovers_via_retry_on_conflict() {
    let Some((pool, dept_id)) = pool_with_default_dept().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping ticket_number_collision_recovers_via_retry_on_conflict");
        return;
    };

    // Pre-seed a ticket holding a specific number (the collision target).
    let taken: i64 = TICKET_NUMBER_MIN + 1; // 100001
    let fresh: i64 = TICKET_NUMBER_MAX - 1; // 999998 (assume free)
    // Clear any stragglers from prior runs so the numbers are predictable.
    for n in [taken, fresh] {
        sqlx::query(r#"DELETE FROM ticket WHERE "ticketID" = $1"#)
            .bind(n)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query(r#"INSERT INTO ticket ("ticketID", dept_id, email) VALUES ($1, $2, $3)"#)
        .bind(taken)
        .bind(dept_id)
        .bind("taken@example.com")
        .execute(&pool)
        .await
        .unwrap();

    // Feed `taken` first (forces a UNIQUE violation on the INSERT), then `fresh`
    // (the re-roll that succeeds). A `Cell` counter drives the sequence and lets
    // us assert the generator was called twice — i.e. the INSERT was retried.
    let calls = Cell::new(0u32);
    let seq = [taken, fresh];
    let input = NewTicket::validated(sample_input()).unwrap();
    let ticket = create_ticket_with_numbers(&pool, &input, || {
        let i = calls.get();
        calls.set(i + 1);
        seq[i as usize % seq.len()]
    })
    .await
    .expect("create recovers from the forced collision");

    assert_eq!(calls.get(), 2, "the INSERT was retried once (re-roll), no SELECT");
    assert_eq!(ticket.ticket_number, fresh, "the second (fresh) number won");

    cleanup(&pool, &ticket).await;
    sqlx::query(r#"DELETE FROM ticket WHERE "ticketID" = $1"#)
        .bind(taken)
        .execute(&pool)
        .await
        .unwrap();
}
