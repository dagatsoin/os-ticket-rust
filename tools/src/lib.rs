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

// --- TS-M2-A2: attachment + helpdesk config keys -------------------------
//
// @implements FS-022.13 / FS-032 (ref): the attachment config keys the upload
//   gate (TS-M2-A2 `validate_upload`) and the create/reply paths read.
// @implements ROADMAP M2 Decisions §6: `helpdesk_url` base URL for %{url}.

/// Master switch for attachments (FS-022.13). Stored as `"true"`/`"false"`.
pub const CFG_ALLOW_ATTACHMENTS: &str = "allow_attachments";
/// Comma-separated extension allow-list (BS-022.13); `.*` allows all.
pub const CFG_ALLOWED_FILETYPES: &str = "allowed_filetypes";
/// Maximum accepted upload size in bytes (FS-022.13).
pub const CFG_MAX_FILE_SIZE: &str = "max_file_size";
/// Base helpdesk URL the substitution engine reads for `%{url}` (§6).
pub const CFG_HELPDESK_URL: &str = "helpdesk_url";

/// Pinned default: attachments enabled.
pub const DEFAULT_ALLOW_ATTACHMENTS: &str = "true";
/// Pinned default allow-list.
pub const DEFAULT_ALLOWED_FILETYPES: &str = ".pdf,.png,.jpg,.txt,.doc";
/// Pinned default max upload size: 1 MiB.
pub const DEFAULT_MAX_FILE_SIZE: &str = "1048576";
/// Pinned default helpdesk base URL (the Vite SPA origin).
pub const DEFAULT_HELPDESK_URL: &str = "http://localhost:3702";

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

    // --- TS-M2-A2: attachment + helpdesk config keys (idempotent) ----------
    seed_m2_config(&mut tx).await?;

    tx.commit().await?;

    Ok(SeedResult {
        dept_id,
        group_id,
        staff_id,
    })
}

/// Seed the four M2 config keys with their pinned defaults, idempotently.
///
/// `INSERT ... ON CONFLICT (key) DO NOTHING` so a re-run leaves the existing
/// value untouched (and never duplicates a row) — these are admin-tunable
/// defaults, not reference literals to overwrite. A fresh DB gets the pinned
/// values; an existing DB keeps whatever an admin has since set.
///
/// @implements FS-022.13 / ROADMAP §6: attachment + helpdesk config seed (AC-5).
pub async fn seed_m2_config(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> anyhow::Result<()> {
    for (key, value) in [
        (CFG_ALLOW_ATTACHMENTS, DEFAULT_ALLOW_ATTACHMENTS),
        (CFG_ALLOWED_FILETYPES, DEFAULT_ALLOWED_FILETYPES),
        (CFG_MAX_FILE_SIZE, DEFAULT_MAX_FILE_SIZE),
        (CFG_HELPDESK_URL, DEFAULT_HELPDESK_URL),
    ] {
        sqlx::query(
            "INSERT INTO config (key, value) VALUES ($1, $2)
             ON CONFLICT (key) DO NOTHING",
        )
        .bind(key)
        .bind(value)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
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
