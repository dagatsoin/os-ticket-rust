//! `db` — Postgres connection pool + health probe for the osTicket
//! modernisation backend.
//!
//! @implements FS-001 health concept — the DB reachability probe behind
//! `GET /api/health`. The probe uses a SHORT timeout so a db-down state returns
//! a fast structured response and never hangs the request (TS-M1-A1 AC-3).

use std::time::Duration;

use sqlx::postgres::{PgPool, PgPoolOptions};

/// Default short timeout for the health-check DB ping. Chosen so a db-down
/// `GET /api/health` returns well under a second instead of hanging.
pub const HEALTH_PING_TIMEOUT: Duration = Duration::from_millis(750);

/// Errors that can occur while connecting to Postgres.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("failed to connect to the database: {0}")]
    Connect(#[from] sqlx::Error),
}

/// Build a Postgres connection pool from a connection string.
///
/// A bounded `acquire_timeout` keeps pool checkout from blocking indefinitely
/// (the per-probe ping timeout is applied separately by [`ping`]).
pub async fn connect(database_url: &str) -> Result<PgPool, DbError> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(HEALTH_PING_TIMEOUT)
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// Probe database reachability with a short timeout.
///
/// Returns `true` when a trivial `SELECT 1` round-trips within
/// [`HEALTH_PING_TIMEOUT`]; returns `false` on any error or timeout. It never
/// hangs and never panics, so the caller (the `/api/health` handler) can always
/// respond quickly with `db: "ok" | "down"`.
///
/// @implements FS-001 health concept (TS-M1-A1 AC-2 / AC-3).
pub async fn ping(pool: &PgPool) -> bool {
    let query = sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(pool);
    matches!(
        tokio::time::timeout(HEALTH_PING_TIMEOUT, query).await,
        Ok(Ok(1))
    )
}
