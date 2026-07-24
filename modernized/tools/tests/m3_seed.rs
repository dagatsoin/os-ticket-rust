//! Integration tests for the TS-M3-prep M3 seed data expansion and TS-M3-D2 SLA seeding.
//!
//! DB-backed; skipped (pass with a log line) when `TEST_DATABASE_URL` is unset.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p tools --test m3_seed
//! ```
//!
//! @implements TS-M3-prep: M3 seed data expansion (AC-1..AC-6).
//! @implements TS-M3-D2: SLA plan seeding (AC-1..AC-5).

use sqlx::postgres::PgPool;

fn test_db_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

/// Connect, migrate, and clear M3-specific tables so counts are deterministic.
async fn fresh_pool() -> Option<PgPool> {
    let url = test_db_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    // Clear M3-specific tables + seed-target tables in FK-safe order.
    // Order matters: children before parents, nullify FKs before deleting referenced rows.
    for stmt in [
        // Nullify department sla_id FKs before deleting sla rows
        "UPDATE department SET sla_id = NULL",
        // Nullify help_topic sla_id FKs before deleting sla rows
        "UPDATE help_topic SET sla_id = NULL",
        "DELETE FROM team_member",
        "DELETE FROM help_topic",
        "DELETE FROM team",
        "DELETE FROM ticket_thread",
        "DELETE FROM ticket",
        "DELETE FROM staff",
        "DELETE FROM group_dept_access",
        "DELETE FROM groups",
        "DELETE FROM sla",
        "DELETE FROM department",
        "DELETE FROM config WHERE key IN ('default_ticket_status','default_priority','show_assigned_tickets','show_answered_tickets','ticket_lock_time','max_page_size','default_sla_id')",
    ] {
        sqlx::query(stmt).execute(&pool).await.expect(stmt);
    }
    Some(pool)
}

/// AC-1: The seed creates two departments (Support, Sales).
#[tokio::test]
async fn seed_creates_two_departments() {
    let Some(pool) = fresh_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping seed_creates_two_departments");
        return;
    };

    tools::seed(&pool).await.expect("seed");

    // Query department names ordered by dept_id
    let depts: Vec<String> =
        sqlx::query_scalar("SELECT dept_name FROM department ORDER BY dept_id")
            .fetch_all(&pool)
            .await
            .expect("query departments");

    assert_eq!(depts.len(), 2, "expected exactly two departments");
    assert!(depts.contains(&"Support".to_string()), "Support department must exist");
    assert!(depts.contains(&"Sales".to_string()), "Sales department must exist");
}

/// AC-2: The seed creates one team (Tier 2) with agent as member.
#[tokio::test]
async fn seed_creates_team_with_agent_member() {
    let Some(pool) = fresh_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping seed_creates_team_with_agent_member");
        return;
    };

    tools::seed(&pool).await.expect("seed");

    // Query team with its member
    let result: Vec<(String, String)> = sqlx::query_as(
        "SELECT t.name, s.username
         FROM team t
         JOIN team_member tm ON t.team_id = tm.team_id
         JOIN staff s ON tm.staff_id = s.staff_id",
    )
    .fetch_all(&pool)
    .await
    .expect("query team members");

    assert_eq!(result.len(), 1, "expected exactly one team membership");
    let (team_name, username) = &result[0];
    assert_eq!(team_name, "Tier 2", "team name must be 'Tier 2'");
    assert_eq!(username, "agent", "team member must be 'agent'");

    // Also verify the agent is the team lead
    let lead_username: Option<String> = sqlx::query_scalar(
        "SELECT s.username FROM team t JOIN staff s ON t.lead_id = s.staff_id WHERE t.name = 'Tier 2'",
    )
    .fetch_optional(&pool)
    .await
    .expect("query team lead");

    assert_eq!(
        lead_username.as_deref(),
        Some("agent"),
        "agent must be the team lead"
    );
}

/// AC-3: The seed creates two help topics (General, Billing) with full column values.
/// @implements TS-M3-D2 AC-4: Billing topic has Urgent SLA (not null).
#[tokio::test]
async fn seed_creates_help_topics() {
    let Some(pool) = fresh_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping seed_creates_help_topics");
        return;
    };

    tools::seed(&pool).await.expect("seed");

    // Query help topics with all required columns
    #[allow(clippy::type_complexity)]
    let topics: Vec<(String, bool, bool, bool, i32, Option<i32>, Option<i32>, i32, i32)> =
        sqlx::query_as(
            "SELECT topic, isactive, ispublic, noautoresp, priority_id, dept_id, sla_id, page_id, sort
             FROM help_topic ORDER BY topic_id",
        )
        .fetch_all(&pool)
        .await
        .expect("query help topics");

    assert_eq!(topics.len(), 2, "expected exactly two help topics");

    // Get department IDs for verification
    let support_dept_id: i32 =
        sqlx::query_scalar("SELECT dept_id FROM department WHERE dept_name = 'Support'")
            .fetch_one(&pool)
            .await
            .expect("Support department must exist");
    let sales_dept_id: i32 =
        sqlx::query_scalar("SELECT dept_id FROM department WHERE dept_name = 'Sales'")
            .fetch_one(&pool)
            .await
            .expect("Sales department must exist");

    // Get Urgent SLA id for verification (TS-M3-D2 assigns Urgent to Billing)
    let urgent_sla_id: i32 =
        sqlx::query_scalar("SELECT id FROM sla WHERE name = 'Urgent'")
            .fetch_one(&pool)
            .await
            .expect("Urgent SLA must exist");

    // Verify "General" topic (first by topic_id)
    let general = &topics[0];
    assert_eq!(general.0, "General", "first topic must be 'General'");
    assert!(general.1, "General isactive must be true");
    assert!(general.2, "General ispublic must be true");
    assert!(!general.3, "General noautoresp must be false");
    assert_eq!(general.4, 2, "General priority_id must be 2");
    assert_eq!(general.5, Some(support_dept_id), "General dept_id must be Support");
    assert_eq!(general.6, None, "General sla_id must be null (inherits from dept)");
    assert_eq!(general.7, 0, "General page_id must be 0");
    assert_eq!(general.8, 1, "General sort must be 1");

    // Verify "Billing" topic (second by topic_id) — has Urgent SLA per TS-M3-D2
    let billing = &topics[1];
    assert_eq!(billing.0, "Billing", "second topic must be 'Billing'");
    assert!(billing.1, "Billing isactive must be true");
    assert!(billing.2, "Billing ispublic must be true");
    assert!(!billing.3, "Billing noautoresp must be false");
    assert_eq!(billing.4, 2, "Billing priority_id must be 2");
    assert_eq!(billing.5, Some(sales_dept_id), "Billing dept_id must be Sales");
    assert_eq!(billing.6, Some(urgent_sla_id), "Billing sla_id must be Urgent SLA (TS-M3-D2 AC-4)");
    assert_eq!(billing.7, 0, "Billing page_id must be 0");
    assert_eq!(billing.8, 2, "Billing sort must be 2");
}

/// AC-4: The agent's group has access to both departments.
#[tokio::test]
async fn agent_group_has_access_to_both_departments() {
    let Some(pool) = fresh_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping agent_group_has_access_to_both_departments");
        return;
    };

    tools::seed(&pool).await.expect("seed");

    // Query departments the agent's group has access to
    let depts: Vec<String> = sqlx::query_scalar(
        "SELECT d.dept_name
         FROM group_dept_access gda
         JOIN department d ON gda.dept_id = d.dept_id
         WHERE gda.group_id = (SELECT group_id FROM staff WHERE username = 'agent')",
    )
    .fetch_all(&pool)
    .await
    .expect("query group dept access");

    assert_eq!(depts.len(), 2, "expected access to two departments");
    assert!(depts.contains(&"Support".to_string()), "must have access to Support");
    assert!(depts.contains(&"Sales".to_string()), "must have access to Sales");
}

/// AC-5: The config keys for M3 are seeded.
#[tokio::test]
async fn seed_creates_m3_config_keys() {
    let Some(pool) = fresh_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping seed_creates_m3_config_keys");
        return;
    };

    tools::seed(&pool).await.expect("seed");

    // Query M3 config keys
    let configs: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM config
         WHERE key IN ('show_assigned_tickets', 'show_answered_tickets', 'ticket_lock_time', 'max_page_size')
         ORDER BY key",
    )
    .fetch_all(&pool)
    .await
    .expect("query config keys");

    assert_eq!(configs.len(), 4, "expected exactly four M3 config keys");

    // Verify values (sorted by key alphabetically)
    let expected = vec![
        ("max_page_size".to_string(), "25".to_string()),
        ("show_answered_tickets".to_string(), "0".to_string()),
        ("show_assigned_tickets".to_string(), "1".to_string()),
        ("ticket_lock_time".to_string(), "2".to_string()),
    ];

    assert_eq!(configs, expected, "config values must match expected");
}

/// AC-6: The agent's group has the required permission flags.
#[tokio::test]
async fn agent_group_has_required_permission_flags() {
    let Some(pool) = fresh_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping agent_group_has_required_permission_flags");
        return;
    };

    tools::seed(&pool).await.expect("seed");

    // Query permission flags for the agent's group
    let (can_close, can_delete, can_assign, can_transfer, can_edit): (bool, bool, bool, bool, bool) =
        sqlx::query_as(
            "SELECT can_close_tickets, can_delete_tickets, can_assign_tickets, can_transfer_tickets, can_edit_tickets
             FROM groups
             WHERE group_id = (SELECT group_id FROM staff WHERE username = 'agent')",
        )
        .fetch_one(&pool)
        .await
        .expect("query group permissions");

    assert!(can_close, "can_close_tickets must be true");
    assert!(can_delete, "can_delete_tickets must be true");
    assert!(can_assign, "can_assign_tickets must be true");
    assert!(can_transfer, "can_transfer_tickets must be true");
    assert!(can_edit, "can_edit_tickets must be true");
}

/// Idempotency: the seed can be run multiple times without duplicating M3 data.
#[tokio::test]
async fn seed_m3_is_idempotent() {
    let Some(pool) = fresh_pool().await else {
        eprintln!("TEST_DATABASE_URL unset — skipping seed_m3_is_idempotent");
        return;
    };

    // First seed
    tools::seed(&pool).await.expect("first seed");

    let dept_count_1: i64 = sqlx::query_scalar("SELECT count(*) FROM department")
        .fetch_one(&pool)
        .await
        .unwrap();
    let team_count_1: i64 = sqlx::query_scalar("SELECT count(*) FROM team")
        .fetch_one(&pool)
        .await
        .unwrap();
    let topic_count_1: i64 = sqlx::query_scalar("SELECT count(*) FROM help_topic")
        .fetch_one(&pool)
        .await
        .unwrap();

    // Second seed
    tools::seed(&pool).await.expect("second seed (idempotent)");

    let dept_count_2: i64 = sqlx::query_scalar("SELECT count(*) FROM department")
        .fetch_one(&pool)
        .await
        .unwrap();
    let team_count_2: i64 = sqlx::query_scalar("SELECT count(*) FROM team")
        .fetch_one(&pool)
        .await
        .unwrap();
    let topic_count_2: i64 = sqlx::query_scalar("SELECT count(*) FROM help_topic")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(dept_count_1, dept_count_2, "department count must not change");
    assert_eq!(team_count_1, team_count_2, "team count must not change");
    assert_eq!(topic_count_1, topic_count_2, "help_topic count must not change");
    assert_eq!(dept_count_2, 2, "must have exactly 2 departments");
    assert_eq!(team_count_2, 1, "must have exactly 1 team");
    assert_eq!(topic_count_2, 2, "must have exactly 2 help topics");
}
