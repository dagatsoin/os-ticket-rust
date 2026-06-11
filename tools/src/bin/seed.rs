//! `seed` binary — the M1 dev-reset command (TS-M1-A3).
//!
//! Loads `backend/.env` (or the shell `DATABASE_URL`), ensures migrations are
//! applied, then runs the idempotent seed. Safe to re-run.
//!
//! ```sh
//! DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_dev \
//!   cargo run -p tools --bin seed
//! ```
//!
//! @implements BS-091: idempotent seed task / M1 dev-reset.

use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Pick up backend/.env if present, else rely on the shell environment.
    let _ = dotenvy::from_filename("backend/.env");
    let _ = dotenvy::dotenv();

    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be set (e.g. postgres://postgres:pass123@localhost:5432/osticket_dev)")?;

    let pool = db::connect(&database_url)
        .await
        .context("connecting to the database")?;

    // Ensure the schema exists so a fresh DB can be seeded in one command.
    db::migrate(&pool).await.context("applying migrations")?;

    let result = tools::seed(&pool).await.context("seeding fixtures")?;

    println!(
        "seed ok: department#{} group#{} staff#{} ({} / {})",
        result.dept_id,
        result.group_id,
        result.staff_id,
        tools::STAFF_USERNAME,
        tools::STAFF_PASSWORD,
    );
    Ok(())
}
