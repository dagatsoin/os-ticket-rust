//! `seed` binary — the dev seed / reset command (TS-M1-A3, TS-M2-prep).
//!
//! Loads `backend/.env` (or the shell `DATABASE_URL`), ensures migrations are
//! applied, then runs the idempotent seed. Safe to re-run.
//!
//! ```sh
//! # Non-destructive: upsert the dept/group/agent baseline + config (M1 contract).
//! DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_dev \
//!   cargo run -p tools --bin seed
//!
//! # Destructive dev reset (TS-M2-prep): purge ticket-scoped data, then re-seed.
//! # Refuses any DB that is not a recognised dev DB (osticket_dev/osticket_test).
//! DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_dev \
//!   cargo run -p tools --bin seed -- --reset
//! ```
//!
//! @implements BS-091: idempotent seed task.
//! @implements TS-M2-prep: `--reset` ticket-data purge (truncation +
//!   orphan-blob reclamation), preserving the seeded canned `policy.txt`.

use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Pick up backend/.env if present, else rely on the shell environment.
    let _ = dotenvy::from_filename("backend/.env");
    let _ = dotenvy::dotenv();

    // Minimal arg parse: the only flag is `--reset`. Anything else is a usage error.
    let mut reset = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--reset" => reset = true,
            "-h" | "--help" => {
                println!("usage: seed [--reset]\n  --reset  purge ticket-scoped data (dev DB only), then re-seed");
                return Ok(());
            }
            other => anyhow::bail!("unknown argument `{other}` (usage: seed [--reset])"),
        }
    }

    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be set (e.g. postgres://postgres:pass123@localhost:5432/osticket_dev)")?;

    let pool = db::connect(&database_url)
        .await
        .context("connecting to the database")?;

    // Ensure the schema exists so a fresh DB can be seeded in one command.
    db::migrate(&pool).await.context("applying migrations")?;

    let result = if reset {
        // Destructive path: purge ticket-scoped data, then re-seed the baseline.
        // `reset` refuses any non-dev database before truncating anything.
        let r = tools::reset(&pool)
            .await
            .context("resetting dev database")?;
        println!(
            "reset ok: purged {} → reseeded department#{} group#{} staff#{} ({} / {}); \
             orphan blobs reclaimed (seeded canned policy.txt preserved)",
            tools::RESET_TRUNCATE_TABLES.join(", "),
            r.dept_id,
            r.group_id,
            r.staff_id,
            tools::STAFF_USERNAME,
            tools::STAFF_PASSWORD,
        );
        r
    } else {
        tools::seed(&pool).await.context("seeding fixtures")?
    };

    if !reset {
        println!(
            "seed ok: department#{} group#{} staff#{} ({} / {})",
            result.dept_id,
            result.group_id,
            result.staff_id,
            tools::STAFF_USERNAME,
            tools::STAFF_PASSWORD,
        );
    }
    Ok(())
}
