//! Env-gated developer endpoints (TS-M1-A4b).
//!
//! @implements FS-040: `GET /api/dev/mailbox` surfaces the stub mailer's
//!   recorded sends so qa-criterion-tester can verify reply notifications
//!   without a real SMTP transport. **Disabled in production** (returns 404 so
//!   the route's existence is not even advertised).

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use http::StatusCode;

use ost_core::ApiError;

use crate::state::AppState;

/// `GET /api/dev/mailbox` — return the recorded stub-mailer sends as JSON.
///
/// In production (`APP_ENV=production`) the dev endpoints are disabled and this
/// returns a 404 error envelope, so the endpoint is invisible there.
///
/// @implements FS-040: dev mailbox (dev-only).
pub async fn mailbox(State(state): State<AppState>) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let recorded = state.mailer.recorded();
    (StatusCode::OK, Json(recorded)).into_response()
}
