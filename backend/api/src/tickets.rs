//! Public ticket routes — the web-form adapter over the shared ticket core
//! (TS-M1-B2).
//!
//! @implements BS-011: `POST /api/tickets` — validate + sanitise a public
//!   submission, then create the ticket atomically via [`ost_core::create_ticket`].
//! @implements FS-011.8: required-field + email validation at the web boundary.
//!
//! M1 deviations (deliberate, per the ticket + ROADMAP Decisions §2):
//! * `topicId` is NOT required (deferred to M4).
//! * No CSRF on this route — it is a public, unauthenticated endpoint.

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use http::StatusCode;
use serde::Deserialize;

use ost_core::ticket::{create_ticket, NewTicket, NewTicketInput};
use ost_core::validation::FieldError;
use ost_core::ApiError;

use crate::state::AppState;

/// Public create-ticket request body. The external contract uses `message` for
/// the first message body (the core models it as `body`); the field name is
/// translated in [`create_public_ticket`] so validation errors surface under the
/// `message` key the client submitted.
#[derive(Debug, Deserialize)]
pub struct CreateTicketRequest {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub message: String,
}

/// `POST /api/tickets` — create a ticket from a public web submission.
///
/// Validates and sanitises the payload (FS-011.8 minus `topicId`), then creates
/// the ticket + its first `M` thread entry atomically via the shared core. On
/// success returns **201** with the generated 6-digit ticket number. On a
/// validation failure returns **422** with the shared error envelope and a
/// per-field map (no ticket is created).
///
/// CSRF-exempt and unauthenticated (ROADMAP Decisions §2).
///
/// @implements BS-011 / FS-011.8: public create + validation.
pub async fn create_public_ticket(
    State(state): State<AppState>,
    Json(req): Json<CreateTicketRequest>,
) -> Result<Response, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Ticket creation is unavailable"))?;

    let input = NewTicketInput {
        email: req.email,
        name: req.name,
        subject: req.subject,
        body: req.message,
        source: Some("Web".to_string()),
        dept_id: None,
    };

    let new_ticket = NewTicket::validated(input).map_err(map_field_errors)?;

    let ticket = create_ticket(pool, &new_ticket)
        .await
        .map_err(|_| ApiError::internal("Could not create the ticket"))?;

    let body = Json(serde_json::json!({
        "ticketNumber": ticket.ticket_number,
    }));
    Ok((StatusCode::CREATED, body).into_response())
}

/// Map the core's per-field validation errors into a 422 [`ApiError`] envelope.
///
/// Translates the core's internal `body` field name back to the public
/// `message` key the client submitted so the field map mirrors the request shape.
fn map_field_errors(errors: Vec<(String, FieldError)>) -> ApiError {
    let mut err = ApiError::validation("Please correct the highlighted fields");
    for (field, fe) in errors {
        let public_field = if field == "body" { "message" } else { &field };
        err = err.with_field(public_field, fe.to_string());
    }
    err
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_errors_map_body_to_message_key() {
        let errors = vec![
            ("email".to_string(), FieldError::InvalidEmail),
            ("body".to_string(), FieldError::Missing),
        ];
        let api = map_field_errors(errors);
        assert_eq!(api.status, StatusCode::UNPROCESSABLE_ENTITY);
        // The internal `body` field is surfaced under the public `message` key.
        assert!(api.fields.contains_key("message"), "body→message remap");
        assert!(!api.fields.contains_key("body"));
        assert!(api.fields.contains_key("email"));
    }
}
