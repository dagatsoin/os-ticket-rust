//! Client-realm read routes (TS-M1-D1).
//!
//! The client portal is account-less: a [`ClientSession`] (TS-M1-A4b) is scoped
//! to exactly one ticket (its external number + bound email). This route returns
//! that one ticket's thread — and **only its `M` (client message) and `R` (staff
//! response) entries**. Internal notes (`N`) are NEVER exposed to a client.
//!
//! @implements FS-010 (security): client thread excludes internal `N` notes.
//! @implements BS-010: a client reads only its own (session-bound) ticket.

use axum::extract::State;
use axum::Json;
use serde::Serialize;
use sqlx::Row;

use ost_core::attachment::load_attachments_by_ref;
use ost_core::{ApiError, AttachmentView};

use crate::auth::realm::ClientSession;
use crate::state::AppState;

/// One thread entry returned to a client (only `M` / `R` ever reach here).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientThreadEntry {
    pub id: i64,
    /// `M` (client message) or `R` (staff response) — never `N`.
    pub thread_type: String,
    pub poster: String,
    pub body: String,
    /// Attachments bound to this entry — `[{id, name, size, mime}]` (§7), empty
    /// when none. The `id` is the download route's `attachmentId`.
    pub attachments: Vec<AttachmentView>,
}

/// The client's own ticket plus its (M/R-only) thread.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientTicket {
    pub number: i64,
    pub subject: String,
    pub status: String,
    pub created: String,
    /// Thread entries in chronological order (created ASC), `M` + `R` only.
    pub entries: Vec<ClientThreadEntry>,
}

/// `GET /api/client/ticket` — return the logged-in client's bound ticket + thread.
///
/// The ticket is determined solely by the session (the `ost_client_sess` cookie
/// binds one ticket number); there is no path parameter, so a client can only
/// ever read its own ticket. The thread is filtered to `M` + `R` in SQL —
/// internal `N` notes are never returned (security-critical, FS-010).
///
/// @implements FS-010 (security): M+R only, never N.
/// @implements BS-010: session-scoped single-ticket read.
pub async fn ticket(
    State(state): State<AppState>,
    session: ClientSession,
) -> Result<Json<ClientTicket>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Ticket view is unavailable"))?;

    // Resolve the session's bound ticket by its external number (the session is
    // ticket-scoped; the email is part of the bound identity).
    let header: Option<(i64, i64, String, String, String)> = sqlx::query_as(
        r#"SELECT ticket_id, "ticketID", subject, status,
                  to_char(created, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created
           FROM ticket
           WHERE "ticketID" = $1 AND lower(email) = lower($2)"#,
    )
    .bind(session.ticket_number)
    .bind(&session.email)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal("Ticket lookup failed"))?;

    let (ticket_id, number, subject, status, created) =
        header.ok_or_else(|| ApiError::not_found("Ticket not found"))?;

    // Filter to M + R in SQL so an internal N note can never reach the client,
    // even if one exists on the ticket. created ASC (= id ASC) chronological.
    let rows = sqlx::query(
        "SELECT id, thread_type, poster, body FROM ticket_thread
         WHERE ticket_id = $1 AND thread_type IN ('M', 'R')
         ORDER BY id ASC",
    )
    .bind(ticket_id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Thread lookup failed"))?;

    // Per-entry attachments (§7), keyed by the entry id (= ticket_attachment.ref_id).
    // Only M/R entries reach the client, so an N note's attachments are never
    // surfaced (the SQL above already filtered N out of the entry set).
    let mut by_ref = load_attachments_by_ref(pool, ticket_id)
        .await
        .map_err(|_| ApiError::internal("Attachment lookup failed"))?;

    let entries = rows
        .into_iter()
        .map(|row| {
            let id: i64 = row.get("id");
            ClientThreadEntry {
                attachments: by_ref.remove(&id).unwrap_or_default(),
                id,
                thread_type: row.get("thread_type"),
                poster: row.get("poster"),
                body: row.get("body"),
            }
        })
        .collect();

    Ok(Json(ClientTicket {
        number,
        subject,
        status,
        created,
        entries,
    }))
}
