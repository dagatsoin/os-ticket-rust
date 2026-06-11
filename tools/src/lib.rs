//! Idempotent seed fixtures for the M1 slice (TS-M1-A3).
//!
//! @implements BS-091: seed fixtures (dept + group + staff) + reference defaults.
//! @implements FS-091.3: status literal `open` reference default.
//! @implements FS-091.5: priority literal `normal` reference default.
//! @implements BS-091.8: group → department access seeded.
//!
//! Modernisation note: this is the stand-in for the legacy installer's
//! default-data seeding (FS-060) and first-run admin bootstrap. It is a Rust
//! task (not pure SQL) because the staff password must be argon2id-hashed with
//! the TS-M1-A4a util, which cannot run inside SQL. Every write is an idempotent
//! upsert, so re-running is the M1 dev-reset mechanism (AC-4).

use sqlx::postgres::PgPool;

/// Seeded department name.
pub const DEPT_NAME: &str = "Support";
/// Seeded permission group name.
pub const GROUP_NAME: &str = "M1 Agents";
/// Seeded staff username (documented QA credential).
pub const STAFF_USERNAME: &str = "agent";
/// Seeded staff plaintext password (documented QA credential).
pub const STAFF_PASSWORD: &str = "Agent123!";
/// FS-091.3 status literal seeded as a config default.
pub const STATUS_OPEN: &str = "open";
/// FS-091.5 priority literal seeded as a config default.
pub const PRIORITY_NORMAL: &str = "normal";

/// Config key holding the default ticket status literal.
pub const CFG_DEFAULT_STATUS: &str = "default_ticket_status";
/// Config key holding the default priority literal.
pub const CFG_DEFAULT_PRIORITY: &str = "default_priority";

/// What the seed produced/confirmed, returned for logging and tests.
#[derive(Debug, Clone, Copy)]
pub struct SeedResult {
    pub dept_id: i32,
    pub group_id: i32,
    pub staff_id: i32,
}

/// Seed (or re-confirm) the M1 fixture set against `pool`, idempotently.
///
/// Upserts exactly one department, one permission group (with the M1 flags +
/// department access), one staff account (argon2id-hashed password), and the
/// status/priority reference defaults. Re-running yields the same single rows.
///
/// @implements BS-091: idempotent seed (TS-M1-A3 AC-1..AC-5).
pub async fn seed(pool: &PgPool) -> anyhow::Result<SeedResult> {
    let mut tx = pool.begin().await?;

    // --- department (unique dept_name → ON CONFLICT upsert) ----------------
    let dept_id: i32 = sqlx::query_scalar(
        "INSERT INTO department (dept_name, ispublic) VALUES ($1, true)
         ON CONFLICT (dept_name) DO UPDATE SET updated = now()
         RETURNING dept_id",
    )
    .bind(DEPT_NAME)
    .fetch_one(&mut *tx)
    .await?;

    // --- groups (no unique name column → look up by name, then insert) -----
    // The schema does not enforce group-name uniqueness (FS-091: groups has no
    // name-unique), so emulate upsert-by-name to stay idempotent.
    let existing_group: Option<i32> =
        sqlx::query_scalar("SELECT group_id FROM groups WHERE group_name = $1")
            .bind(GROUP_NAME)
            .fetch_optional(&mut *tx)
            .await?;
    let group_id: i32 = match existing_group {
        Some(id) => {
            sqlx::query(
                "UPDATE groups
                   SET group_enabled = true,
                       can_create_tickets = true,
                       can_post_reply = true,
                       updated = now()
                 WHERE group_id = $1",
            )
            .bind(id)
            .execute(&mut *tx)
            .await?;
            id
        }
        None => {
            sqlx::query_scalar(
                "INSERT INTO groups
                   (group_name, group_enabled, can_create_tickets, can_post_reply)
                 VALUES ($1, true, true, true)
                 RETURNING group_id",
            )
            .bind(GROUP_NAME)
            .fetch_one(&mut *tx)
            .await?
        }
    };

    // --- group → department access (composite PK → idempotent upsert) ------
    sqlx::query(
        "INSERT INTO group_dept_access (group_id, dept_id) VALUES ($1, $2)
         ON CONFLICT (group_id, dept_id) DO NOTHING",
    )
    .bind(group_id)
    .bind(dept_id)
    .execute(&mut *tx)
    .await?;

    // --- staff (unique username → upsert; hash via TS-M1-A4a) --------------
    let passwd = ost_core::hash_password(STAFF_PASSWORD)?;
    let staff_id: i32 = sqlx::query_scalar(
        "INSERT INTO staff
           (group_id, dept_id, username, firstname, lastname, passwd, email, isactive)
         VALUES ($1, $2, $3, 'Agent', 'One', $4, 'agent@example.com', true)
         ON CONFLICT (username) DO UPDATE
           SET group_id = EXCLUDED.group_id,
               dept_id  = EXCLUDED.dept_id,
               passwd   = EXCLUDED.passwd,
               updated  = now()
         RETURNING staff_id",
    )
    .bind(group_id)
    .bind(dept_id)
    .bind(STAFF_USERNAME)
    .bind(&passwd)
    .fetch_one(&mut *tx)
    .await?;

    // --- reference defaults the M1 flows read (FS-091 literals) -------------
    upsert_config(&mut tx, CFG_DEFAULT_STATUS, STATUS_OPEN).await?;
    upsert_config(&mut tx, CFG_DEFAULT_PRIORITY, PRIORITY_NORMAL).await?;

    tx.commit().await?;

    Ok(SeedResult {
        dept_id,
        group_id,
        staff_id,
    })
}

/// Idempotently upsert a single config key/value pair.
async fn upsert_config(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    key: &str,
    value: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO config (key, value) VALUES ($1, $2)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated = now()",
    )
    .bind(key)
    .bind(value)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
