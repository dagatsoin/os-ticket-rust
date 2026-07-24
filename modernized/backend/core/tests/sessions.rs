//! Integration tests for the TS-M1-A4b DB-backed session store + group
//! permission lookup. Run against a live Postgres test DB; skipped (pass, with a
//! log line) when `TEST_DATABASE_URL` is unset so offline / DB-less CI stays
//! green:
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p ost_core --test sessions
//! ```
//!
//! @implements FS-002.14: DB-backed sessions, TTL, fixation defence (AC-3).
//! @implements BS-001: group-permission lookup backing the permission gate (AC-4).

use std::time::Duration;

use ost_core::permission::PERM_CAN_POST_REPLY;
use ost_core::session::{SessionData, SessionStore};
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

/// AC-3 (regeneration): two consecutive logins (creates) mint DISTINCT session
/// ids, so the post-login cookie value never equals a prior one.
#[tokio::test]
async fn session_id_is_regenerated_on_each_login() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping session_id_is_regenerated_on_each_login");
        return;
    };
    let store = SessionStore::new(pool);
    let data = SessionData::Staff { staff_id: 1, sort_prefs: None };

    let first = store.create(&data).await.expect("first session");
    let second = store.create(&data).await.expect("second session");
    assert_ne!(first.id, second.id, "a fresh login must mint a new session id");

    // Both load back correctly while live.
    assert!(store.load(&first.id).await.unwrap().is_some());
    assert!(store.load(&second.id).await.unwrap().is_some());

    store.destroy(&first.id).await.unwrap();
    store.destroy(&second.id).await.unwrap();
}

/// AC-3 (TTL): a row whose expiry is already in the past is treated as
/// expired/invalid by `load` (returns `None`), while a row inside its 86400s
/// window loads fine.
#[tokio::test]
async fn expired_session_is_treated_as_invalid() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping expired_session_is_treated_as_invalid");
        return;
    };
    let store = SessionStore::new(pool);
    let data = SessionData::Client {
        ticket_number: 424242,
        email: "ttl@example.com".into(),
    };

    // An already-expired row (expiry 10s in the past).
    let expired_id = store
        .create_with_expiry(&data, Duration::from_secs(10), true)
        .await
        .expect("seed expired row");
    assert!(
        store.load(&expired_id).await.unwrap().is_none(),
        "an expired session must not load"
    );

    // A row still within its window loads.
    let live = store.create(&data).await.expect("live session");
    assert!(store.load(&live.id).await.unwrap().is_some());

    store.destroy(&expired_id).await.unwrap();
    store.destroy(&live.id).await.unwrap();
}

/// A client (ticket-scoped) session round-trips its ticket number + email.
#[tokio::test]
async fn client_session_round_trips_ticket_scope() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping client_session_round_trips_ticket_scope");
        return;
    };
    let store = SessionStore::new(pool);
    let data = SessionData::Client {
        ticket_number: 555111,
        email: "client@example.com".into(),
    };
    let created = store.create(&data).await.expect("create client session");
    let loaded = store.load(&created.id).await.unwrap().expect("loads");
    assert_eq!(loaded.data, data);
    store.destroy(&created.id).await.unwrap();
}

/// AC-4 (DB path): a staff member whose group grants `can_post_reply` is granted
/// by the named gate; a staff member whose group lacks it is denied. The rows
/// are created in a self-contained way (avoiding a dep cycle on the `tools`
/// crate, which depends on `ost_core`).
#[tokio::test]
async fn staff_group_permission_lookup_drives_named_gate() {
    let Some(pool) = migrated_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping staff_group_permission_lookup_drives_named_gate");
        return;
    };

    let suffix = unique_suffix();
    let dept_id: i32 =
        sqlx::query_scalar("INSERT INTO department (dept_name) VALUES ($1) RETURNING dept_id")
            .bind(format!("perm-dept-{suffix}"))
            .fetch_one(&pool)
            .await
            .unwrap();

    // A group that grants can_post_reply, and one that does not.
    let granting_group: i32 = sqlx::query_scalar(
        "INSERT INTO groups (group_name, group_enabled, can_post_reply) \
         VALUES ($1, true, true) RETURNING group_id",
    )
    .bind(format!("perm-grant-{suffix}"))
    .fetch_one(&pool)
    .await
    .unwrap();
    let denying_group: i32 = sqlx::query_scalar(
        "INSERT INTO groups (group_name, group_enabled, can_post_reply) \
         VALUES ($1, true, false) RETURNING group_id",
    )
    .bind(format!("perm-deny-{suffix}"))
    .fetch_one(&pool)
    .await
    .unwrap();

    let granted_staff: i32 = sqlx::query_scalar(
        "INSERT INTO staff (group_id, dept_id, username, passwd) \
         VALUES ($1, $2, $3, 'x') RETURNING staff_id",
    )
    .bind(granting_group)
    .bind(dept_id)
    .bind(format!("perm-grant-{suffix}"))
    .fetch_one(&pool)
    .await
    .unwrap();
    let denied_staff: i32 = sqlx::query_scalar(
        "INSERT INTO staff (group_id, dept_id, username, passwd) \
         VALUES ($1, $2, $3, 'x') RETURNING staff_id",
    )
    .bind(denying_group)
    .bind(dept_id)
    .bind(format!("perm-deny-{suffix}"))
    .fetch_one(&pool)
    .await
    .unwrap();

    let store = SessionStore::new(pool.clone());
    let granted = store
        .staff_group_permissions(granted_staff)
        .await
        .unwrap()
        .expect("granting staff has a group");
    assert!(granted.has(PERM_CAN_POST_REPLY), "group grants can_post_reply");

    let denied = store
        .staff_group_permissions(denied_staff)
        .await
        .unwrap()
        .expect("denying staff has a group");
    assert!(!denied.has(PERM_CAN_POST_REPLY), "group lacks can_post_reply");

    // An unknown staff id yields no permissions.
    assert!(store.staff_group_permissions(-1).await.unwrap().is_none());

    // Cleanup (FK order).
    sqlx::query("DELETE FROM staff WHERE staff_id IN ($1, $2)")
        .bind(granted_staff)
        .bind(denied_staff)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM groups WHERE group_id IN ($1, $2)")
        .bind(granting_group)
        .bind(denying_group)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM department WHERE dept_id = $1")
        .bind(dept_id)
        .execute(&pool)
        .await
        .unwrap();
}

/// A cheap unique-ish suffix so concurrent test runs don't collide on unique
/// names (no external crate needed).
fn unique_suffix() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
