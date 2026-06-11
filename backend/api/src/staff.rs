//! Staff-realm read/profile routes (TS-M1-C1/C2/C3).
//!
//! All routes here are gated by the staff realm (the [`StaffSession`] extractor,
//! TS-M1-A4b); mutating routes additionally enforce CSRF + a named permission.
//!
//! @implements BS-002: authenticated staff profile (`GET /api/staff/me`).

use axum::extract::{FromRequest, Multipart, Path, Request, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use ost_core::attachment::load_attachments_by_ref;
use ost_core::mailer::{Mailer, OutboundMail};
use ost_core::permission::PERM_CAN_POST_REPLY;
use ost_core::ticket::{
    append_thread_entry, append_thread_entry_with_attachment, load_thread, NewThreadEntry,
};
use ost_core::{ApiError, AttachmentSpec, AttachmentView};

use crate::attachments::{drain_multipart, load_upload_policy, validate_attachment};
use crate::auth::gate::require_staff_permission;
use crate::auth::realm::StaffSession;
use crate::auth::StaffCsrf;
use crate::state::AppState;

/// The authenticated staff profile returned by `GET /api/staff/me`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffMe {
    pub id: i32,
    pub username: String,
    /// Display name (`firstname lastname`, trimmed; falls back to username).
    pub name: String,
    pub dept_id: i32,
}

/// `GET /api/staff/me` — return the authenticated staff account profile.
///
/// Read-only (no CSRF). The staff realm gate yields a 401 when no valid staff
/// session is present.
///
/// @implements BS-002: staff profile lookup `{ id, username, name, deptId }`.
pub async fn me(
    State(state): State<AppState>,
    session: StaffSession,
) -> Result<Json<StaffMe>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Profile lookup is unavailable"))?;

    let row: Option<(String, String, String, i32)> = sqlx::query_as(
        "SELECT username, firstname, lastname, dept_id FROM staff WHERE staff_id = $1",
    )
    .bind(session.staff_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal("Profile lookup failed"))?;

    // A valid session whose account no longer exists is treated as invalid.
    let (username, firstname, lastname, dept_id) =
        row.ok_or_else(|| ApiError::unauthenticated("Session expired or invalid"))?;

    let name = display_name(&firstname, &lastname, &username);

    Ok(Json(StaffMe {
        id: session.staff_id,
        username,
        name,
        dept_id,
    }))
}

/// One row of the staff open-tickets queue.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueItem {
    /// Internal record key (the detail route's `:id` path param).
    pub id: i64,
    /// External 6-digit ticket number.
    pub number: i64,
    pub subject: String,
    pub email: String,
    /// ISO-8601 creation timestamp.
    pub created: String,
}

/// `GET /api/staff/tickets` — list **open** tickets, newest first.
///
/// M1 scope: no department scoping, no tabs/search; the single seeded agent sees
/// every open ticket. Sort is `created DESC` (BS-020).
///
/// @implements BS-020: open-tickets queue (number + subject + email + created,
///   created DESC).
pub async fn list_tickets(
    State(state): State<AppState>,
    _session: StaffSession,
) -> Result<Json<Vec<QueueItem>>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Ticket queue is unavailable"))?;

    // created DESC, with id DESC as a stable tiebreaker for same-instant rows.
    // `created` is rendered to ISO-8601 text in SQL (the sqlx build excludes the
    // chrono/time features), so it decodes straight to a String.
    let rows: Vec<(i64, i64, String, String, String)> = sqlx::query_as(
        r#"SELECT ticket_id, "ticketID", subject, email,
                  to_char(created, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created
           FROM ticket
           WHERE status = 'open'
           ORDER BY created DESC, ticket_id DESC"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Ticket queue lookup failed"))?;

    let items = rows
        .into_iter()
        .map(|(id, number, subject, email, created)| QueueItem {
            id,
            number,
            subject,
            email,
            created,
        })
        .collect();
    Ok(Json(items))
}

/// One thread entry as returned in the staff ticket detail.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadEntryView {
    pub id: i64,
    /// `M` (client message) / `R` (staff response) / `N` (internal note).
    pub thread_type: String,
    pub poster: String,
    pub body: String,
    /// Attachments bound to this entry — `[{id, name, size, mime}]` (§7), empty
    /// when none. The `id` is the `ticket_attachment` binding id (the download
    /// route's `attachmentId`).
    pub attachments: Vec<AttachmentView>,
}

/// A single staff ticket detail: the ticket header + its full thread.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketDetail {
    pub id: i64,
    pub number: i64,
    pub subject: String,
    pub email: String,
    pub name: String,
    pub status: String,
    pub created: String,
    /// Thread entries in chronological order (created ASC), **all** types.
    pub entries: Vec<ThreadEntryView>,
}

/// `GET /api/staff/tickets/{id}` — one ticket plus its full thread.
///
/// Returns thread entries in chronological order (created ASC) **including all
/// types M/R/N** — staff see internal notes (the client thread route excludes
/// `N`). 404 when no ticket has that internal id.
///
/// @implements BS-021: ticket detail with ordered full thread (M/R/N, ASC).
pub async fn ticket_detail(
    State(state): State<AppState>,
    _session: StaffSession,
    Path(id): Path<i64>,
) -> Result<Json<TicketDetail>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Ticket detail is unavailable"))?;

    let header: Option<(i64, i64, String, String, String, String, String)> =
        sqlx::query_as(
            r#"SELECT ticket_id, "ticketID", subject, email, name, status,
                      to_char(created, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created
               FROM ticket WHERE ticket_id = $1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("Ticket detail lookup failed"))?;

    let (ticket_id, number, subject, email, name, status, created) =
        header.ok_or_else(|| ApiError::not_found("Ticket not found"))?;

    let detail = build_detail(
        pool, ticket_id, number, subject, email, name, status, created,
    )
    .await?;
    Ok(Json(detail))
}

/// Load a ticket's full thread and assemble a [`TicketDetail`] from a header row.
///
/// Shared by [`ticket_detail`] and [`reply`] so both return the identical shape
/// (the staff thread: all M/R/N entries, created ASC).
#[allow(clippy::too_many_arguments)]
async fn build_detail(
    pool: &sqlx::postgres::PgPool,
    ticket_id: i64,
    number: i64,
    subject: String,
    email: String,
    name: String,
    status: String,
    created: String,
) -> Result<TicketDetail, ApiError> {
    // The shared core loads the thread in created ASC order, all M/R/N entries.
    let thread = load_thread(pool, ticket_id)
        .await
        .map_err(|_| ApiError::internal("Thread lookup failed"))?;

    // Per-entry attachments (§7), keyed by the entry id (= ticket_attachment.ref_id).
    let mut by_ref = load_attachments_by_ref(pool, ticket_id)
        .await
        .map_err(|_| ApiError::internal("Attachment lookup failed"))?;

    let entries = thread
        .into_iter()
        .map(|e| ThreadEntryView {
            attachments: by_ref.remove(&e.id).unwrap_or_default(),
            id: e.id,
            thread_type: e.thread_type,
            poster: e.poster,
            body: e.body,
        })
        .collect();

    Ok(TicketDetail {
        id: ticket_id,
        number,
        subject,
        email,
        name,
        status,
        created,
        entries,
    })
}

/// A staff reply request body (the `application/json` variant).
#[derive(Debug, Deserialize)]
pub struct ReplyRequest {
    /// The reply text (sanitised by the shared core before persist).
    pub body: String,
}

/// The reply's parsed inputs from either accepted shape.
///
/// `canned_id` is accepted from the multipart form and IGNORED for now — D4
/// wires the canned-response reuse. It is captured (not dropped on parse) so the
/// D4 follow-up can read it without changing the wire contract.
struct ReplyInput {
    body: String,
    /// Optional `cannedId` form part — accepted + IGNORED in A5 (D4 consumes it).
    _canned_id: Option<String>,
    /// Optional `attachment` file part (already validated before binding).
    attachment: Option<AttachmentSpec>,
}

/// `POST /api/staff/tickets/{id}/reply` — append a staff response (`R`).
///
/// Gated by the staff realm + CSRF (the [`StaffCsrf`] extractor) and the
/// `can_post_reply` named permission. **Dual-accepts by `Content-Type` (§2):**
/// `application/json` `{body}` keeps the M1 contract (no attachment);
/// `multipart/form-data` carries `body` (+ an optional `cannedId` part, accepted
/// and IGNORED here — D4 wires it) plus an optional `attachment` file part bound
/// to the new `R` entry (ref_type `R`). A rejected attachment ⇒ 422 (keyed on
/// `attachment`) and posts NO reply.
///
/// A **pure append** in M1: the ticket status is NOT mutated (isanswered
/// semantics deferred). The intended client notification is recorded by the stub
/// mailer **only after the DB append commits**. Returns the updated thread (the
/// same shape as the detail route, including each entry's `attachments`).
///
/// @implements BS-021: append `R`, pure append (no status mutation).
/// @implements FS-021.3 / FS-021.16: optional attachment bound to the `R` entry.
/// @implements FS-040: record the client notification only after commit.
pub async fn reply(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i64>,
    request: Request,
) -> Result<Json<TicketDetail>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Reply is unavailable"))?;

    // Permission gate: a session lacking can_post_reply is denied 403.
    require_staff_permission(&state, &session, PERM_CAN_POST_REPLY).await?;

    // Parse the two accepted shapes into a common ReplyInput.
    let input = parse_reply_input(request, &state).await?;

    // Resolve the ticket header (404 when the id is unknown) + the acting agent's
    // display name (the response poster).
    let header: Option<(i64, i64, String, String, String, String, String)> =
        sqlx::query_as(
            r#"SELECT ticket_id, "ticketID", subject, email, name, status,
                      to_char(created, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created
               FROM ticket WHERE ticket_id = $1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("Ticket lookup failed"))?;
    let (ticket_id, number, subject, email, name, status, created) =
        header.ok_or_else(|| ApiError::not_found("Ticket not found"))?;

    // Validate the attachment (A2) BEFORE appending — a rejected file posts no
    // reply (FS-021.16). 422 keyed on `attachment`.
    if input.attachment.is_some() {
        let policy = load_upload_policy(pool).await?;
        validate_attachment(&input.attachment, &policy)?;
    }

    let agent = staff_display_name(pool, session.staff_id).await?;

    // Append the R entry via the shared core (it commits on success). Only AFTER
    // a successful commit do we record the mailer intent.
    let entry = NewThreadEntry::response(agent, Some(session.staff_id), &input.body);
    match input.attachment {
        Some(spec) => {
            append_thread_entry_with_attachment(pool, ticket_id, &entry, &state.store, &spec)
                .await
                .map_err(|_| ApiError::internal("Could not append the reply"))?;
        }
        None => {
            append_thread_entry(pool, ticket_id, &entry)
                .await
                .map_err(|_| ApiError::internal("Could not append the reply"))?;
        }
    }

    // Mailer intent recorded post-commit (stub records, does not send).
    let mail = OutboundMail {
        to: email.clone(),
        subject: format!("Ticket #{number} updated"),
        body: entry.body.clone(),
    };
    let _ = state.mailer.send(mail);

    let detail = build_detail(
        pool, ticket_id, number, subject, email, name, status, created,
    )
    .await?;
    Ok(Json(detail))
}

/// Parse a reply request body into [`ReplyInput`], dual-accepting JSON or
/// multipart by `Content-Type` (§2).
async fn parse_reply_input(request: Request, state: &AppState) -> Result<ReplyInput, ApiError> {
    let is_multipart = request
        .headers()
        .get(http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| {
            ct.trim_start()
                .to_ascii_lowercase()
                .starts_with("multipart/form-data")
        })
        .unwrap_or(false);

    if is_multipart {
        let multipart = Multipart::from_request(request, state)
            .await
            .map_err(|_| ApiError::validation("Malformed multipart request"))?;
        let form = drain_multipart(multipart).await?;
        Ok(ReplyInput {
            body: form.field("body").to_string(),
            // `cannedId` is accepted + IGNORED here (D4 wires the canned reuse).
            _canned_id: form.fields.get("cannedId").cloned(),
            attachment: form.attachment,
        })
    } else {
        let Json(req) = Json::<ReplyRequest>::from_request(request, state)
            .await
            .map_err(|_| ApiError::validation("Malformed JSON request"))?;
        Ok(ReplyInput {
            body: req.body,
            _canned_id: None,
            attachment: None,
        })
    }
}

/// Resolve a staff member's display name for use as a thread poster.
async fn staff_display_name(
    pool: &sqlx::postgres::PgPool,
    staff_id: i32,
) -> Result<String, ApiError> {
    let row: Option<(String, String, String)> =
        sqlx::query_as("SELECT username, firstname, lastname FROM staff WHERE staff_id = $1")
            .bind(staff_id)
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::internal("Staff lookup failed"))?;
    let (username, firstname, lastname) =
        row.ok_or_else(|| ApiError::unauthenticated("Session expired or invalid"))?;
    Ok(display_name(&firstname, &lastname, &username))
}

/// Compose a display name from first/last, falling back to the username when
/// both name parts are blank.
fn display_name(firstname: &str, lastname: &str, username: &str) -> String {
    let full = format!("{} {}", firstname.trim(), lastname.trim());
    let full = full.trim().to_string();
    if full.is_empty() {
        username.to_string()
    } else {
        full
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_composes_or_falls_back() {
        assert_eq!(display_name("Agent", "One", "agent"), "Agent One");
        assert_eq!(display_name("", "", "agent"), "agent");
        assert_eq!(display_name("  ", "Solo", "agent"), "Solo");
    }
}
