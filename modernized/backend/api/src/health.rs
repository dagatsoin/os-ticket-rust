//! `GET /api/health` handler.
//!
//! @implements FS-001 health concept (TS-M1-A1 AC-2 / AC-3) — reports DB
//! reachability via a short-timeout ping so a db-down state returns a fast
//! structured `{ "status": "ok", "db": "down" }` response and never hangs.

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::state::AppState;

/// Health response body: `{ "status": "ok", "db": "ok" | "down" }`.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub db: &'static str,
}

/// Health handler. Always returns HTTP 200 with `status: "ok"`; the `db` field
/// reflects the short-timeout DB ping (`"ok"` reachable, `"down"` otherwise).
pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let db = match &state.pool {
        Some(pool) if db::ping(pool).await => "ok",
        _ => "down",
    };
    Json(HealthResponse { status: "ok", db })
}
