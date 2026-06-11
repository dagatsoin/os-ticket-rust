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
use ost_core::canned::{load_offerable_response, load_ticket_vars};
use ost_core::permission::PERM_CAN_POST_REPLY;
use ost_core::ticket::{load_thread, post_staff_reply, NewThreadEntry};
use ost_core::variable::VariableReplacer;
use ost_core::{ApiError, AttachmentSpec, AttachmentView};

use crate::attachments::{drain_multipart, load_upload_policy, validate_attachment};
use crate::auth::gate::require_staff_permission;
use crate::auth::realm::StaffSession;
use crate::auth::StaffCsrf;
use crate::canned::{CFG_HELPDESK_URL, DEFAULT_HELPDESK_URL};
use crate::config_keys::read_config;
use crate::state::AppState;

/// A ticket header row as selected by the detail + reply routes:
/// `(ticket_id, number, subject, email, name, status, created, isanswered)`.
type TicketHeader = (i64, i64, String, String, String, String, String, bool);

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
    /// Whether the ticket has been answered (any staff reply ⇒ true, §3). Drives
    /// the queue's Answered / Unanswered badge.
    pub isanswered: bool,
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
    let rows: Vec<(i64, i64, String, String, String, bool)> = sqlx::query_as(
        r#"SELECT ticket_id, "ticketID", subject, email,
                  to_char(created, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created,
                  isanswered
           FROM ticket
           WHERE status = 'open'
           ORDER BY created DESC, ticket_id DESC"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Ticket queue lookup failed"))?;

    let items = rows
        .into_iter()
        .map(|(id, number, subject, email, created, isanswered)| QueueItem {
            id,
            number,
            subject,
            email,
            created,
            isanswered,
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
    /// Whether the ticket has been answered (any staff reply ⇒ true, §3). Drives
    /// the detail view's Answered / Unanswered badge.
    pub isanswered: bool,
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

    let header: Option<TicketHeader> =
        sqlx::query_as(
            r#"SELECT ticket_id, "ticketID", subject, email, name, status,
                      to_char(created, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created,
                      isanswered
               FROM ticket WHERE ticket_id = $1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("Ticket detail lookup failed"))?;

    let (ticket_id, number, subject, email, name, status, created, isanswered) =
        header.ok_or_else(|| ApiError::not_found("Ticket not found"))?;

    let detail = build_detail(
        pool, ticket_id, number, subject, email, name, status, created, isanswered,
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
    isanswered: bool,
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
        isanswered,
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
/// `canned_id` is the optional `cannedId` multipart form part — when present the
/// reply re-renders that canned response's body server-side and carries its
/// attachments onto the new `R` entry (D4).
struct ReplyInput {
    body: String,
    /// Optional `cannedId` form part (multipart only) — D4 canned-response reuse.
    canned_id: Option<i32>,
    /// Optional `attachment` file part (already validated before binding).
    attachment: Option<AttachmentSpec>,
}

/// `POST /api/staff/tickets/{id}/reply` — append a staff response (`R`).
///
/// Gated by the staff realm + CSRF (the [`StaffCsrf`] extractor) and the
/// `can_post_reply` named permission. **Dual-accepts by `Content-Type` (§2):**
/// `application/json` `{body}` keeps the M1 contract (no attachment / no canned);
/// `multipart/form-data` carries `body`, an optional `cannedId` part, and an
/// optional own `attachment` file part.
///
/// Canned reuse (D4): when `cannedId` references a canned response offerable for
/// this ticket (enabled + dept 0/all or the ticket's dept), the posted body is
/// re-rendered **server-side** from that canned response (substituted for this
/// ticket + `helpdesk_url`, for integrity) and its attachments are carried onto
/// the new `R` entry **by file id** (no re-upload, D1 dedup). An own uploaded
/// file is validated/stored/bound alongside. A disabled / unknown / out-of-scope
/// `cannedId` ⇒ 404; a rejected own attachment ⇒ 422 (keyed `attachment`); both
/// post NO reply.
///
/// isanswered (§3): ANY staff reply — plain, own-file, or canned-assisted —
/// marks the ticket `isanswered = true` (the M1-faithful semantics), authored by
/// the posting agent (documented divergence from legacy's SYSTEM actor). The
/// client reply notification (notice wrapper + packaged template, TS-M2-E3) is
/// sent through the active mailer only after the DB commit. Returns the updated
/// thread (the detail shape, with `isanswered` + each entry's `attachments`).
///
/// @implements BS-021: append `R`.
/// @implements FS-022.14: canned reuse — substituted body + carried attachments.
/// @implements ROADMAP §3: any staff reply marks the ticket answered.
/// @implements FS-021.3 / FS-021.16: optional own attachment bound to the entry.
/// @implements FS-021.3: send the client reply notification only after commit.
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
    let header: Option<TicketHeader> =
        sqlx::query_as(
            r#"SELECT ticket_id, "ticketID", subject, email, name, status,
                      to_char(created, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created,
                      isanswered
               FROM ticket WHERE ticket_id = $1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("Ticket lookup failed"))?;
    let (ticket_id, number, subject, email, name, status, created, _isanswered) =
        header.ok_or_else(|| ApiError::not_found("Ticket not found"))?;

    // Validate the own attachment (A2) BEFORE appending — a rejected file posts
    // no reply (FS-021.16). 422 keyed on `attachment`.
    if input.attachment.is_some() {
        let policy = load_upload_policy(pool).await?;
        validate_attachment(&input.attachment, &policy)?;
    }

    // Resolve the canned response (if any): re-render its body server-side for
    // this ticket and collect its attachment file ids to carry. A disabled /
    // unknown / out-of-scope cannedId ⇒ 404 (BS-022.2), posting no reply.
    let mut reply_body = input.body.clone();
    let mut canned_file_ids: Vec<i64> = Vec::new();
    if let Some(canned_id) = input.canned_id {
        let vars = load_ticket_vars(pool, ticket_id)
            .await
            .map_err(|_| ApiError::internal("Ticket lookup failed"))?
            .ok_or_else(|| ApiError::not_found("Ticket not found"))?;
        let canned = load_offerable_response(pool, canned_id, vars.dept_id)
            .await
            .map_err(|_| ApiError::internal("Canned response lookup failed"))?
            .ok_or_else(|| ApiError::not_found("Canned response not found"))?;
        let base_url = read_config(pool, CFG_HELPDESK_URL)
            .await?
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_HELPDESK_URL.to_string());
        let replacer = VariableReplacer::new(vars.to_context(), base_url);
        reply_body = replacer.render(&canned.body);
        canned_file_ids = canned.file_ids;
    }

    let agent = staff_display_name(pool, session.staff_id).await?;

    // Append the R entry + bind own/canned attachments + mark answered, all in
    // one tx (post_staff_reply commits on success). Author stays the agent.
    let entry = NewThreadEntry::response(agent, Some(session.staff_id), &reply_body);
    post_staff_reply(
        pool,
        ticket_id,
        &entry,
        &state.store,
        input.attachment.as_ref(),
        &canned_file_ids,
    )
    .await
    .map_err(|_| ApiError::internal("Could not append the reply"))?;

    // After the commit, send the staff-reply notification to the requester via
    // the notice wrapper + packaged template (always-send in M2, §12). Mail
    // failure never fails the already-committed reply (TS-M2-E3).
    crate::email_wiring::send_reply_notification(&state, pool, ticket_id).await;

    // Re-read the detail (now isanswered = true) so the response reflects §3.
    let detail = build_detail(
        pool, ticket_id, number, subject, email, name, status, created, true,
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
        // Parse the optional `cannedId` (a blank/absent part ⇒ no canned). A
        // present-but-non-numeric value is a malformed request (422).
        let canned_id = match form.fields.get("cannedId").map(|s| s.trim()) {
            Some(s) if !s.is_empty() => Some(
                s.parse::<i32>()
                    .map_err(|_| ApiError::validation("cannedId must be an integer"))?,
            ),
            _ => None,
        };
        Ok(ReplyInput {
            body: form.field("body").to_string(),
            canned_id,
            attachment: form.attachment,
        })
    } else {
        let Json(req) = Json::<ReplyRequest>::from_request(request, state)
            .await
            .map_err(|_| ApiError::validation("Malformed JSON request"))?;
        Ok(ReplyInput {
            body: req.body,
            canned_id: None,
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
