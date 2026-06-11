//! Authorized attachment download routes (TS-M2-B1).
//!
//! Streams a stored blob back to a requester whose realm session can access the
//! attachment's **parent ticket** (DEVIATION D2 — BS-022.8 behaviour preserved
//! without the legacy session-bound MD5 hash). Two routes (ROADMAP §8):
//!
//! * `GET /api/client/ticket/attachments/{attachmentId}` — client realm,
//!   **session-bound, no ticketId param** (the session already pins the ticket).
//!   An attachment on an `N` note is invisible to a client (404) — a client never
//!   sees internal notes.
//! * `GET /api/staff/tickets/{ticketId}/attachments/{attachmentId}` — staff
//!   realm; the explicit `ticketId` must match the attachment's parent ticket.
//!
//! Any cross-ticket / unknown / inaccessible attachment returns **404 with no
//! body** — indistinguishable from a genuinely-missing id, so the route leaks no
//! existence information (§8).
//!
//! @implements FS-022.10: authorized attachment download.
//! @implements FS-022.11: download delivery (Content-Disposition + stored MIME).
//! @implements BS-022.8 / EC-022.7 / EC-022.9 (D2): parent-ticket session gate;
//!   bad/unknown id and cross-ticket access both 404, no existence leak.

use axum::body::Body;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use http::{header, StatusCode};
use tokio_util::io::ReaderStream;

use ost_core::attachment::{load_download_file, DownloadFile};
use ost_core::ticket::ThreadType;
use ost_core::ApiError;

use crate::auth::realm::{ClientSession, StaffSession};
use crate::state::AppState;

/// `GET /api/client/ticket/attachments/{attachmentId}` — stream a blob the
/// client's session-bound ticket owns. 404 (no body, no existence leak) for an
/// unknown id, another ticket's attachment, or an `N`-note attachment.
///
/// @implements FS-022.10 / BS-022.8 (D2): client parent-ticket gate.
pub async fn client_download(
    State(state): State<AppState>,
    session: ClientSession,
    Path(attachment_id): Path<i64>,
) -> Response {
    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return not_found(),
    };

    // Resolve the client session's bound ticket id (number + email identity).
    let session_ticket_id: Option<i64> = match sqlx::query_scalar(
        r#"SELECT ticket_id FROM ticket
           WHERE "ticketID" = $1 AND lower(email) = lower($2)"#,
    )
    .bind(session.ticket_number)
    .bind(&session.email)
    .fetch_optional(pool)
    .await
    {
        Ok(v) => v,
        Err(_) => return not_found(),
    };
    let Some(session_ticket_id) = session_ticket_id else {
        return not_found();
    };

    let file = match resolve_file(pool, attachment_id).await {
        Some(f) => f,
        None => return not_found(),
    };

    // Parent-ticket gate: the binding must belong to the session's ticket.
    if file.ticket_id != session_ticket_id {
        return not_found();
    }
    // A client never sees internal notes — an attachment on an `N` entry is 404.
    if file.ref_type == ThreadType::Note.as_db() {
        return not_found();
    }

    stream_file(&state, file).await
}

/// `GET /api/staff/tickets/{ticketId}/attachments/{attachmentId}` — stream a blob
/// on the named ticket. 404 (no body) for an unknown id or an attachment whose
/// parent ticket is not `ticketId`.
///
/// @implements FS-022.10 / BS-022.8 (D2): staff parent-ticket gate.
pub async fn staff_download(
    State(state): State<AppState>,
    _session: StaffSession,
    Path((ticket_id, attachment_id)): Path<(i64, i64)>,
) -> Response {
    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return not_found(),
    };

    let file = match resolve_file(pool, attachment_id).await {
        Some(f) => f,
        None => return not_found(),
    };

    // The path's ticketId must be the attachment's actual parent ticket.
    if file.ticket_id != ticket_id {
        return not_found();
    }

    stream_file(&state, file).await
}

/// Resolve a `ticket_attachment` binding id to its [`DownloadFile`], swallowing
/// any DB error into `None` so the route 404s rather than 500s on a lookup glitch
/// (the response stays a no-leak 404 either way).
async fn resolve_file(pool: &sqlx::postgres::PgPool, binding_id: i64) -> Option<DownloadFile> {
    load_download_file(pool, binding_id).await.ok().flatten()
}

/// Stream a resolved blob with the §7 delivery headers (Content-Type from the
/// stored MIME, Content-Length, Content-Disposition attachment + filename). A
/// missing-on-disk blob 404s (no body) like any other failure.
///
/// @implements FS-022.11: streamed delivery with a Content-Disposition filename.
async fn stream_file(state: &AppState, file: DownloadFile) -> Response {
    let handle = match state.store.open(&file.hash).await {
        Ok(f) => f,
        // The metadata row exists but the blob is gone — treat as not found (no
        // existence leak; the requester learns nothing beyond "no bytes").
        Err(_) => return not_found(),
    };

    let stream = ReaderStream::new(handle);
    let body = Body::from_stream(stream);

    let disposition = content_disposition(&file.name);
    let mime = if file.mime.trim().is_empty() {
        ost_core::DEFAULT_MIME.to_string()
    } else {
        file.mime.clone()
    };

    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_DISPOSITION, disposition);
    // Content-Length is advisory; set it from the stored size when sane.
    if file.size >= 0 {
        response = response.header(header::CONTENT_LENGTH, file.size.to_string());
    }
    match response.body(body) {
        Ok(r) => r,
        Err(_) => ApiError::internal("Could not stream the attachment").into_response(),
    }
}

/// A no-body 404 (the shared envelope), used for every unauthorized/unknown case
/// so they are indistinguishable (§8 no existence leak).
fn not_found() -> Response {
    ApiError::not_found("Not found").into_response()
}

/// Build an RFC 6266-safe `Content-Disposition` value for `name`.
///
/// The ASCII `filename="..."` (control chars / quotes / backslashes stripped) is
/// always emitted; when the name has non-ASCII chars a `filename*=UTF-8''…`
/// percent-encoded form is appended so clients with unicode support get the full
/// name. Always `attachment` (download, never inline).
fn content_disposition(name: &str) -> String {
    let ascii_fallback: String = name
        .chars()
        .map(|c| {
            if c.is_ascii() && !c.is_control() && c != '"' && c != '\\' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let ascii_fallback = if ascii_fallback.trim().is_empty() {
        "download".to_string()
    } else {
        ascii_fallback
    };

    if name.is_ascii() {
        format!("attachment; filename=\"{ascii_fallback}\"")
    } else {
        let encoded = percent_encode_rfc5987(name);
        format!("attachment; filename=\"{ascii_fallback}\"; filename*=UTF-8''{encoded}")
    }
}

/// Percent-encode per RFC 5987 (the `filename*` attr value charset): keep the
/// unreserved `attr-char` set, percent-encode everything else byte-by-byte.
fn percent_encode_rfc5987(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let keep = b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'!' | b'#' | b'$' | b'&' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
            );
        if keep {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&format!("{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_filename_is_quoted() {
        assert_eq!(
            content_disposition("invoice.pdf"),
            "attachment; filename=\"invoice.pdf\""
        );
    }

    #[test]
    fn quotes_and_control_chars_are_sanitised() {
        let cd = content_disposition("a\"b\\c.txt");
        assert!(cd.starts_with("attachment; filename=\"a_b_c.txt\""));
        assert!(!cd.contains('\\'));
    }

    #[test]
    fn non_ascii_name_gets_filename_star() {
        let cd = content_disposition("reçu.pdf");
        // ASCII fallback present…
        assert!(cd.contains("filename=\"re_u.pdf\""), "{cd}");
        // …plus the percent-encoded UTF-8 form.
        assert!(cd.contains("filename*=UTF-8''re%C3%A7u.pdf"), "{cd}");
    }

    #[test]
    fn empty_name_falls_back_to_download() {
        assert_eq!(
            content_disposition(""),
            "attachment; filename=\"download\""
        );
    }
}
