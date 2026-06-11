//! Staff-realm read/profile routes (TS-M1-C1/C2/C3).
//!
//! All routes here are gated by the staff realm (the [`StaffSession`] extractor,
//! TS-M1-A4b); mutating routes additionally enforce CSRF + a named permission.
//!
//! @implements BS-002: authenticated staff profile (`GET /api/staff/me`).

use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;

use ost_core::ticket::load_thread;
use ost_core::ApiError;

use crate::auth::realm::StaffSession;
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

    // The shared core loads the thread in created ASC order, all M/R/N entries.
    let entries = load_thread(pool, ticket_id)
        .await
        .map_err(|_| ApiError::internal("Thread lookup failed"))?
        .into_iter()
        .map(|e| ThreadEntryView {
            id: e.id,
            thread_type: e.thread_type,
            poster: e.poster,
            body: e.body,
        })
        .collect();

    Ok(Json(TicketDetail {
        id: ticket_id,
        number,
        subject,
        email,
        name,
        status,
        created,
        entries,
    }))
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
