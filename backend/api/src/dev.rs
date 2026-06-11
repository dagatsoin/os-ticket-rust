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
use serde::Deserialize;

use ost_core::ticket::{
    append_thread_entry, create_ticket, NewThreadEntry, NewTicket, NewTicketInput, ThreadType,
};
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
    let recorded = state.mailer.recorder.recorded();
    (StatusCode::OK, Json(recorded)).into_response()
}

/// Optional shaping for the dev seed-ticket endpoint.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedTicketRequest {
    /// Also append a staff response (`R`) to the seeded ticket.
    #[serde(default)]
    pub with_reply: bool,
    /// Also append an internal note (`N`) to the seeded ticket.
    #[serde(default)]
    pub with_note: bool,
}

/// `POST /api/dev/seed-ticket` — create an isolated ticket for QA, returning the
/// data the client portal needs to log in: `{ ticketNumber, email }`.
///
/// Disabled in production (returns a 404 envelope, like the dev mailbox) so the
/// endpoint is invisible there. Optionally seeds a staff reply (`R`) and an
/// internal note (`N`) so the M/R-only client-thread security AC has an `N` to
/// prove is excluded. Each call uses a fresh unique email so seeded tickets do
/// not collide on the composite (ticketID, email) identity.
///
/// @implements FS-010 (test infra): isolated ticket fixture for the client E2E.
pub async fn seed_ticket(
    State(state): State<AppState>,
    body: Option<Json<SeedTicketRequest>>,
) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let Json(req) = body.unwrap_or_default();

    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Seeding is unavailable").into_response(),
    };

    // A fresh unique email per seed keeps the (ticketID, email) identity distinct.
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let email = format!("qa+{nonce}@example.com");

    let new_ticket = match NewTicket::validated(NewTicketInput {
        email: email.clone(),
        name: "QA Seed".to_string(),
        subject: "Seeded ticket".to_string(),
        body: "Seeded ticket message.".to_string(),
        source: Some("Web".to_string()),
        dept_id: None,
    }) {
        Ok(t) => t,
        Err(_) => return ApiError::internal("Seed input invalid").into_response(),
    };

    let ticket = match create_ticket(pool, &new_ticket).await {
        Ok(t) => t,
        Err(_) => return ApiError::internal("Could not seed ticket").into_response(),
    };

    if req.with_reply {
        let r = NewThreadEntry::response("QA Agent", None, "Seeded staff reply.");
        if let Err(e) = append_thread_entry(pool, ticket.ticket_id, &r).await {
            tracing::warn!(error = %e, "seed_ticket: reply append failed");
        }
    }
    if req.with_note {
        let n = NewThreadEntry {
            thread_type: ThreadType::Note,
            poster: "QA Agent".to_string(),
            staff_id: None,
            body: "Seeded internal note (must never reach the client).".to_string(),
        };
        if let Err(e) = append_thread_entry(pool, ticket.ticket_id, &n).await {
            tracing::warn!(error = %e, "seed_ticket: note append failed");
        }
    }

    let resp = Json(serde_json::json!({
        "ticketNumber": ticket.ticket_number,
        "email": email,
    }));
    (StatusCode::CREATED, resp).into_response()
}
