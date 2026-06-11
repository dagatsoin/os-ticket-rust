//! Shared application state passed to every handler.

use sqlx::postgres::PgPool;

/// Application state. The pool is optional so the server boots (and the health
/// endpoint / error contract stay testable) even with no live DB.
#[derive(Clone)]
pub struct AppState {
    pub pool: Option<PgPool>,
}

impl AppState {
    /// State with a connected pool.
    pub fn with_pool(pool: PgPool) -> Self {
        Self { pool: Some(pool) }
    }

    /// State with no DB pool (health reports `db: "down"`).
    pub fn without_db() -> Self {
        Self { pool: None }
    }
}
