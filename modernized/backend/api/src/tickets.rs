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

use axum::extract::{FromRequest, Multipart, Path, Request, State};
use axum::response::{IntoResponse, Response};
use axum::Json;
use http::StatusCode;
use serde::Deserialize;

use ost_core::ticket::{create_ticket, create_ticket_with_attachment, NewTicket, NewTicketInput};
use ost_core::validation::FieldError;
use ost_core::workflow::apply_sla_on_create;
use ost_core::{ApiError, AttachmentSpec, AttachmentView};

use crate::attachments::{drain_multipart, load_upload_policy, validate_attachment};
use crate::state::AppState;

/// Whether a request's `Content-Type` is `multipart/form-data` (§2 dual-accept).
fn is_multipart(req: &Request) -> bool {
    req.headers()
        .get(http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.trim_start().to_ascii_lowercase().starts_with("multipart/form-data"))
        .unwrap_or(false)
}

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
/// **Dual-accepts by `Content-Type` (ROADMAP §2):** `application/json` keeps the
/// M1 contract unchanged (no attachment); `multipart/form-data` carries
/// `name`/`email`/`subject`/`message` as individual parts plus an optional
/// `attachment` file part. Fields are validated first (FS-011.8 minus `topicId`),
/// then the attachment against the seeded config policy (A2). On success returns
/// **201** with the generated 6-digit ticket number. A field or attachment
/// validation failure returns **422** with the shared error envelope (file
/// errors keyed on `attachment`); no ticket and no attachment rows are created.
///
/// CSRF-exempt and unauthenticated (ROADMAP Decisions §2). After a successful
/// commit the new-ticket autoresponse is sent to the requester (TS-M2-E3,
/// always-send in M2 §12; suppressed for daemon/postmaster addresses).
///
/// @implements BS-011 / FS-011.8: public create + validation.
/// @implements FS-011.7 / EC-011.5: optional attachment bound to the `M` entry.
/// @implements FS-011.12: new-ticket autoresponse after commit.
pub async fn create_public_ticket(
    State(state): State<AppState>,
    request: Request,
) -> Result<Response, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Ticket creation is unavailable"))?;

    // Parse the two accepted shapes into the same (input, optional attachment).
    let (input, attachment) = if is_multipart(&request) {
        let multipart = Multipart::from_request(request, &state)
            .await
            .map_err(|_| ApiError::validation("Malformed multipart request"))?;
        let form = drain_multipart(multipart).await?;
        let input = NewTicketInput {
            email: form.field("email").to_string(),
            name: form.field("name").to_string(),
            subject: form.field("subject").to_string(),
            body: form.field("message").to_string(),
            source: Some("Web".to_string()),
            dept_id: None,
        };
        (input, form.attachment)
    } else {
        let Json(req) = Json::<CreateTicketRequest>::from_request(request, &state)
            .await
            .map_err(|_| ApiError::validation("Malformed JSON request"))?;
        let input = NewTicketInput {
            email: req.email,
            name: req.name,
            subject: req.subject,
            body: req.message,
            source: Some("Web".to_string()),
            dept_id: None,
        };
        (input, None)
    };

    // Validate fields FIRST, then the attachment (A2) — so a bad field surfaces a
    // field error and we never store a blob for a request that fails validation.
    let new_ticket = NewTicket::validated(input).map_err(map_field_errors)?;

    if attachment.is_some() {
        let policy = load_upload_policy(pool).await?;
        validate_attachment(&attachment, &policy)?;
    }

    let (ticket, _att): (_, Option<AttachmentView>) = match attachment {
        Some(spec) => {
            let (t, view) = create_with_attachment(pool, &state, &new_ticket, spec).await?;
            (t, Some(view))
        }
        None => {
            let t = create_ticket(pool, &new_ticket)
                .await
                .map_err(|_| ApiError::internal("Could not create the ticket"))?;
            (t, None)
        }
    };

    // Apply SLA selection and due-date computation after ticket creation.
    // @implements FS-021.13: SLA selection on ticket create.
    // @implements FS-032.11: Due date computation on create.
    if let Err(e) = apply_sla_on_create(pool, ticket.ticket_id, ticket.dept_id, None).await {
        tracing::warn!(error = %e, "SLA application failed for ticket {}", ticket.ticket_id);
        // Non-fatal: the ticket was created successfully, SLA can be applied later.
    }

    // After a successful commit, send the new-ticket autoresponse to the
    // requester (always-send in M2, §12; loop-suppression from FS-011.12). Mail
    // failure never fails the already-committed create (TS-M2-E3).
    crate::email_wiring::send_new_ticket_autoresponse(&state, pool, ticket.ticket_id).await;

    let body = Json(serde_json::json!({
        "ticketNumber": ticket.ticket_number,
    }));
    Ok((StatusCode::CREATED, body).into_response())
}

/// Create a ticket binding the validated attachment to its `M` entry (A3).
async fn create_with_attachment(
    pool: &sqlx::postgres::PgPool,
    state: &AppState,
    new_ticket: &NewTicket,
    spec: AttachmentSpec,
) -> Result<(ost_core::Ticket, AttachmentView), ApiError> {
    create_ticket_with_attachment(pool, new_ticket, &state.store, &spec)
        .await
        .map_err(|_| ApiError::internal("Could not create the ticket"))
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

/// One thread entry returned in the public thread endpoint (only `M` / `R`).
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicThreadEntry {
    pub id: i64,
    /// `M` (client message) or `R` (staff response) — never `N`.
    #[serde(rename = "type")]
    pub thread_type: String,
    pub poster: String,
    pub body: String,
}

/// `GET /api/tickets/{id}/thread` — return a ticket's thread (public, M/R only).
///
/// Returns thread entries in chronological order (created ASC), filtering out
/// internal notes (`N`). This is the unauthenticated public endpoint.
///
/// @implements BS-021.10: notes excluded from public/client thread endpoint.
pub async fn get_public_thread(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<PublicThreadEntry>>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Thread view is unavailable"))?;

    // Verify ticket exists.
    let exists: Option<(i64,)> = sqlx::query_as("SELECT ticket_id FROM ticket WHERE ticket_id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("Ticket lookup failed"))?;

    if exists.is_none() {
        return Err(ApiError::not_found("Ticket not found"));
    }

    // Load only M/R entries (filter out N notes) in chronological order.
    // @implements BS-021.10: notes excluded from client thread.
    let rows = sqlx::query(
        "SELECT id, thread_type, poster, body FROM ticket_thread
         WHERE ticket_id = $1 AND thread_type IN ('M', 'R')
         ORDER BY id ASC",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Thread lookup failed"))?;

    let entries = rows
        .into_iter()
        .map(|row| {
            use sqlx::Row;
            PublicThreadEntry {
                id: row.get("id"),
                thread_type: row.get("thread_type"),
                poster: row.get("poster"),
                body: row.get("body"),
            }
        })
        .collect();

    Ok(Json(entries))
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
