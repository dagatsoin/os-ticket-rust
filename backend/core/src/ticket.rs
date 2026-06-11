//! Transport-agnostic ticket service core (TS-M1-B1).
//!
//! @implements BS-011: create a ticket — the ticket row **and** its first thread
//!   message in ONE transaction (never a partial ticket).
//! @implements BS-021: append a thread entry (staff response `R`, client
//!   message `M`; internal note `N` is modelled but unused in M1).
//! @implements FS-091.2: external ticket number generation — a random 6-digit
//!   number (100000–999999), collision-safe via the UNIQUE constraint +
//!   retry-on-conflict (insert; on unique-violation re-roll and retry — **no
//!   SELECT-then-INSERT race**).
//! @implements FS-091.4: thread entry type M/R/N.
//!
//! This is the single shared core. The web form (TS-M1-B2), staff reply
//! (TS-M1-C3), and later email/API channels are all thin adapters over the two
//! operations here: [`create_ticket`] and [`append_thread_entry`]. Inputs are
//! validated/sanitised via the TS-M1-A4a utilities before they reach the DB.

use serde::Serialize;
use sqlx::postgres::PgPool;
use sqlx::Row;

use crate::attachment::{insert_attachment, AttachmentSpec, AttachmentView};
use crate::blob::BlobStore;
use crate::sanitize::{safe_html, sanitize};
use crate::validation::{validate_email_field, validate_required, FieldError};

/// Inclusive lower bound of the 6-digit external ticket number (FS-091.2).
pub const TICKET_NUMBER_MIN: i64 = 100_000;
/// Inclusive upper bound of the 6-digit external ticket number (FS-091.2).
pub const TICKET_NUMBER_MAX: i64 = 999_999;
/// Max INSERT retries on a ticket-number unique-violation before giving up.
const MAX_NUMBER_RETRIES: u32 = 16;

/// Field length bounds (mirrors the legacy web-ticket field caps; FS-011.8).
const MAX_EMAIL_LEN: usize = 255;
const MAX_NAME_LEN: usize = 128;
const MAX_SUBJECT_LEN: usize = 255;
const MAX_BODY_LEN: usize = 65_535;

/// Default ticket status literal (FS-091.3).
pub const STATUS_OPEN: &str = "open";

/// A thread entry type (FS-091.4). `Note` is modelled but unused in M1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadType {
    /// `M` — a client message (the first entry of a ticket is always `M`).
    Message,
    /// `R` — a staff response.
    Response,
    /// `N` — an internal note (modelled per FS-091.4; unused in M1).
    Note,
}

impl ThreadType {
    /// The single-character DB literal.
    pub fn as_db(self) -> &'static str {
        match self {
            ThreadType::Message => "M",
            ThreadType::Response => "R",
            ThreadType::Note => "N",
        }
    }
}

/// Validated input to [`create_ticket`]. The raw fields come from a channel
/// adapter (web form / email / API); validation + sanitisation happen in
/// [`NewTicket::validated`].
#[derive(Debug, Clone)]
pub struct NewTicket {
    /// Requester email (validated, trimmed).
    pub email: String,
    /// Requester display name (sanitised, trimmed).
    pub name: String,
    /// Ticket subject (sanitised, trimmed).
    pub subject: String,
    /// First message body (safe-HTML sanitised).
    pub body: String,
    /// Originating source (default `Web`).
    pub source: String,
    /// Optional explicit department; `None` ⇒ the seeded default department.
    pub dept_id: Option<i32>,
}

/// Raw, unvalidated create-ticket fields as received from a channel adapter.
#[derive(Debug, Clone, Default)]
pub struct NewTicketInput {
    pub email: String,
    pub name: String,
    pub subject: String,
    pub body: String,
    pub source: Option<String>,
    pub dept_id: Option<i32>,
}

impl NewTicket {
    /// Validate + sanitise raw input into a [`NewTicket`], or return a per-field
    /// error map (caller maps it to a 422 envelope).
    ///
    /// @implements FS-011.8 / FS-003.10: required-field + length validation and
    ///   HTML sanitisation at the input boundary.
    pub fn validated(input: NewTicketInput) -> Result<Self, Vec<(String, FieldError)>> {
        let mut errors: Vec<(String, FieldError)> = Vec::new();

        let email = match validate_email_field(&input.email, MAX_EMAIL_LEN) {
            Ok(v) => v.to_string(),
            Err(e) => {
                errors.push(("email".into(), e));
                String::new()
            }
        };
        let name = match validate_required(&input.name, MAX_NAME_LEN) {
            Ok(v) => sanitize(v),
            Err(e) => {
                errors.push(("name".into(), e));
                String::new()
            }
        };
        let subject = match validate_required(&input.subject, MAX_SUBJECT_LEN) {
            Ok(v) => sanitize(v),
            Err(e) => {
                errors.push(("subject".into(), e));
                String::new()
            }
        };
        let body = match validate_required(&input.body, MAX_BODY_LEN) {
            Ok(v) => safe_html(v),
            Err(e) => {
                errors.push(("body".into(), e));
                String::new()
            }
        };

        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(NewTicket {
            email,
            name,
            subject,
            body,
            source: input.source.unwrap_or_else(|| "Web".to_string()),
            dept_id: input.dept_id,
        })
    }
}

/// A persisted ticket (the subset the core returns to callers).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ticket {
    /// Internal record key.
    pub ticket_id: i64,
    /// Public external 6-digit number (FS-091.2).
    pub ticket_number: i64,
    /// Owning department.
    pub dept_id: i32,
    /// Status literal (`open` on creation).
    pub status: String,
    /// Requester email.
    pub email: String,
    /// Subject.
    pub subject: String,
}

/// A thread entry as returned to callers (chronological).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThreadEntry {
    pub id: i64,
    pub ticket_id: i64,
    /// `M` / `R` / `N`.
    pub thread_type: String,
    pub poster: String,
    pub body: String,
}

/// Errors from the ticket service core.
#[derive(Debug, thiserror::Error)]
pub enum TicketError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("no default department is configured")]
    NoDefaultDepartment,
    #[error("could not allocate a unique ticket number after {0} attempts")]
    NumberExhausted(u32),
    #[error("ticket not found")]
    NotFound,
}

/// Draw a random 6-digit external ticket number in `100000..=999999`.
///
/// @implements FS-091.2: random-mode 6-digit draw.
pub fn random_ticket_number() -> i64 {
    use password_hash::rand_core::{OsRng, RngCore};
    let span = (TICKET_NUMBER_MAX - TICKET_NUMBER_MIN + 1) as u64; // 900_000
    // Rejection-free modulo is fine here: the slight bias over a 64-bit draw is
    // negligible for a 900k span and does not affect uniqueness/correctness.
    let r = OsRng.next_u64() % span;
    TICKET_NUMBER_MIN + r as i64
}

/// Resolve the default department id (the lowest-id department — the seeded
/// "Support"), used when an input does not specify one.
async fn default_dept_id(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<i32, TicketError> {
    let row: Option<(i32,)> =
        sqlx::query_as("SELECT dept_id FROM department ORDER BY dept_id ASC LIMIT 1")
            .fetch_optional(&mut **tx)
            .await?;
    row.map(|(id,)| id).ok_or(TicketError::NoDefaultDepartment)
}

/// Whether a SQLx error is a unique-constraint violation (Postgres SQLSTATE
/// 23505) — the retry trigger for the ticket-number allocator.
fn is_unique_violation(err: &sqlx::Error) -> bool {
    matches!(
        err,
        sqlx::Error::Database(db) if db.code().as_deref() == Some("23505")
    )
}

/// Create a ticket + its first `M` thread entry in ONE transaction.
///
/// The external number is drawn randomly and the INSERT is retried on a
/// unique-violation (re-roll), so allocation is collision-safe **without a
/// SELECT-then-INSERT race**. On any failure the whole transaction rolls back,
/// so a partial ticket is never persisted (BS-011 atomicity).
///
/// Defaults: `status = open`, department = the input's `dept_id` or the seeded
/// default department.
///
/// @implements BS-011 / FS-091.2 / FS-091.4: atomic create + numbering.
pub async fn create_ticket(pool: &PgPool, input: &NewTicket) -> Result<Ticket, TicketError> {
    create_ticket_with_numbers(pool, input, random_ticket_number).await
}

/// Like [`create_ticket`] but with an injectable number generator.
///
/// The `next_number` closure supplies each candidate external number; on a
/// unique-violation the next call provides the re-roll. Production uses
/// [`random_ticket_number`]; tests inject a deterministic sequence to force the
/// retry-on-conflict path (proving it is the UNIQUE constraint — not a
/// SELECT-then-INSERT — that drives recovery).
///
/// @implements FS-091.2: collision-safe allocation via UNIQUE + retry.
pub async fn create_ticket_with_numbers<F>(
    pool: &PgPool,
    input: &NewTicket,
    next_number: F,
) -> Result<Ticket, TicketError>
where
    F: FnMut() -> i64,
{
    let (ticket, _att) = create_ticket_inner(pool, input, next_number, None).await?;
    Ok(ticket)
}

/// Create a ticket + its first `M` entry, optionally binding ONE attachment to
/// that `M` entry — all in ONE transaction (TS-M2-A3).
///
/// The upload MUST already have passed [`crate::upload::validate_upload`]. Flow
/// (the A3 PINNED order): the ticket row + `M` entry are inserted, the blob is
/// `put` into `store` (content-addressed, dedup), then `attachment_file`
/// (upsert-by-hash) + `ticket_attachment` (`ref_type = M`) are written — all
/// before the single `commit`. Any failure rolls the whole thing back, so a
/// failed attachment leaves NO ticket (and at worst an orphan blob, which D1
/// dedup reclaims).
///
/// Returns the ticket plus the bound attachment's §7 view when one was supplied.
///
/// @implements FS-011.7 / EC-011.5: public create binds an attachment atomically.
pub async fn create_ticket_with_attachment(
    pool: &PgPool,
    input: &NewTicket,
    store: &BlobStore,
    attachment: &AttachmentSpec,
) -> Result<(Ticket, AttachmentView), TicketError> {
    let (ticket, att) =
        create_ticket_inner(pool, input, random_ticket_number, Some((store, attachment))).await?;
    // `att` is `Some` exactly when an attachment was supplied.
    Ok((ticket, att.expect("attachment supplied ⇒ view returned")))
}

/// The shared create implementation: atomic ticket + `M` entry, with an optional
/// attachment bound to that `M` entry inside the same transaction.
async fn create_ticket_inner<F>(
    pool: &PgPool,
    input: &NewTicket,
    mut next_number: F,
    attachment: Option<(&BlobStore, &AttachmentSpec)>,
) -> Result<(Ticket, Option<AttachmentView>), TicketError>
where
    F: FnMut() -> i64,
{
    let mut tx = pool.begin().await?;

    let dept_id = match input.dept_id {
        Some(id) => id,
        None => default_dept_id(&mut tx).await?,
    };

    // Insert the ticket row, re-rolling the number on a unique-violation.
    //
    // Each attempt runs inside a SAVEPOINT: in Postgres a failed statement
    // aborts the surrounding transaction, so on a unique-violation we roll back
    // to the savepoint (clearing the aborted state) and retry the INSERT with a
    // fresh number — still no SELECT-then-INSERT race.
    let mut last_err: Option<sqlx::Error> = None;
    let mut created: Option<(i64, i64)> = None; // (ticket_id, ticket_number)
    for _ in 0..MAX_NUMBER_RETRIES {
        let number = next_number();
        sqlx::query("SAVEPOINT ticket_number_attempt")
            .execute(&mut *tx)
            .await?;
        let res = sqlx::query(
            r#"INSERT INTO ticket ("ticketID", dept_id, email, name, subject, status, source)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING ticket_id, "ticketID""#,
        )
        .bind(number)
        .bind(dept_id)
        .bind(&input.email)
        .bind(&input.name)
        .bind(&input.subject)
        .bind(STATUS_OPEN)
        .bind(&input.source)
        .fetch_one(&mut *tx)
        .await;
        match res {
            Ok(row) => {
                sqlx::query("RELEASE SAVEPOINT ticket_number_attempt")
                    .execute(&mut *tx)
                    .await?;
                let ticket_id: i64 = row.get("ticket_id");
                let ticket_number: i64 = row.get("ticketID");
                created = Some((ticket_id, ticket_number));
                break;
            }
            Err(e) if is_unique_violation(&e) => {
                // Number collided — roll back the aborted statement and re-roll.
                sqlx::query("ROLLBACK TO SAVEPOINT ticket_number_attempt")
                    .execute(&mut *tx)
                    .await?;
                last_err = Some(e);
                continue;
            }
            Err(e) => return Err(e.into()),
        }
    }

    let (ticket_id, ticket_number) = match created {
        Some(t) => t,
        None => {
            // Exhausted retries on persistent collisions.
            if let Some(e) = last_err {
                tracing::warn!(error = %e, "ticket number allocation exhausted retries");
            }
            return Err(TicketError::NumberExhausted(MAX_NUMBER_RETRIES));
        }
    };

    // The first thread entry is always a client message (`M`), holding the body.
    // Capture its id so an attachment (A3) can bind to it in the same tx.
    let m_entry_id: i64 = sqlx::query_scalar(
        "INSERT INTO ticket_thread (ticket_id, thread_type, poster, source, title, body)
         VALUES ($1, 'M', $2, $3, $4, $5)
         RETURNING id",
    )
    .bind(ticket_id)
    .bind(&input.name)
    .bind(&input.source)
    .bind(&input.subject)
    .bind(&input.body)
    .fetch_one(&mut *tx)
    .await?;

    // Optionally bind ONE attachment to the M entry (A3). Store the blob, then
    // write the relational rows — all before the commit so a failure rolls the
    // whole ticket back (EC-011.5: no ticket on a failed attachment).
    let att_view = match attachment {
        Some((store, spec)) => {
            store
                .put(&spec.bytes)
                .await
                .map_err(|e| TicketError::Db(blob_io_to_sqlx(e)))?;
            let view =
                insert_attachment(&mut tx, ticket_id, m_entry_id, ThreadType::Message.as_db(), spec)
                    .await?;
            Some(view)
        }
        None => None,
    };

    tx.commit().await?;

    Ok((
        Ticket {
            ticket_id,
            ticket_number,
            dept_id,
            status: STATUS_OPEN.to_string(),
            email: input.email.clone(),
            subject: input.subject.clone(),
        },
        att_view,
    ))
}

/// Adapt a blob-store I/O error into a `sqlx::Error` so it flows through
/// [`TicketError::Db`] (the create path's single error channel). The blob `put`
/// is a non-DB failure but it must abort the surrounding transaction the same
/// way a DB error would.
fn blob_io_to_sqlx(e: crate::blob::BlobError) -> sqlx::Error {
    sqlx::Error::Io(match e {
        crate::blob::BlobError::Io(io) => io,
        other => std::io::Error::other(other.to_string()),
    })
}

/// Input to [`append_thread_entry`].
#[derive(Debug, Clone)]
pub struct NewThreadEntry {
    pub thread_type: ThreadType,
    /// Who posted (staff name / requester name).
    pub poster: String,
    /// Optional acting staff id (set for `R`/`N`).
    pub staff_id: Option<i32>,
    /// Entry body (safe-HTML sanitised by the caller-facing constructor).
    pub body: String,
}

impl NewThreadEntry {
    /// Build a sanitised staff response (`R`).
    ///
    /// `staff_id` is optional (the `staff_id` column is nullable): pass the
    /// acting staff id when known, or `None`.
    pub fn response(poster: impl Into<String>, staff_id: Option<i32>, body: &str) -> Self {
        Self {
            thread_type: ThreadType::Response,
            poster: sanitize(&poster.into()),
            staff_id,
            body: safe_html(body),
        }
    }

    /// Build a sanitised client message (`M`) appended to an existing ticket.
    pub fn message(poster: impl Into<String>, body: &str) -> Self {
        Self {
            thread_type: ThreadType::Message,
            poster: sanitize(&poster.into()),
            staff_id: None,
            body: safe_html(body),
        }
    }
}

/// Append a thread entry to an existing ticket.
///
/// @implements BS-021 / FS-091.4: append `M`/`R` (and modelled `N`) to a thread.
pub async fn append_thread_entry(
    pool: &PgPool,
    ticket_id: i64,
    entry: &NewThreadEntry,
) -> Result<ThreadEntry, TicketError> {
    // Confirm the ticket exists (FK would catch it, but a clear NotFound is
    // friendlier than a constraint error for the caller).
    let exists: Option<(i64,)> = sqlx::query_as("SELECT ticket_id FROM ticket WHERE ticket_id = $1")
        .bind(ticket_id)
        .fetch_optional(pool)
        .await?;
    if exists.is_none() {
        return Err(TicketError::NotFound);
    }

    let row = sqlx::query(
        "INSERT INTO ticket_thread (ticket_id, thread_type, poster, staff_id, body)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, ticket_id, thread_type, poster, body",
    )
    .bind(ticket_id)
    .bind(entry.thread_type.as_db())
    .bind(&entry.poster)
    .bind(entry.staff_id)
    .bind(&entry.body)
    .fetch_one(pool)
    .await?;

    Ok(ThreadEntry {
        id: row.get("id"),
        ticket_id: row.get("ticket_id"),
        thread_type: row.get("thread_type"),
        poster: row.get("poster"),
        body: row.get("body"),
    })
}

/// Append a thread entry AND bind one attachment to it, in ONE transaction
/// (TS-M2-A5 staff reply).
///
/// The upload MUST already have passed [`crate::upload::validate_upload`]. The
/// entry is inserted, the blob is `put` into `store` (content-addressed dedup),
/// then `attachment_file` (upsert-by-hash) + `ticket_attachment` (`ref_type` =
/// the entry type) are written — all before the single `commit`. A failure rolls
/// the whole thing back, so a rejected attachment posts NO reply (FS-021.16).
///
/// Returns the new entry plus the bound attachment's §7 view.
///
/// @implements FS-021.3 / FS-021.16: staff reply binds an attachment to the new
///   `R` entry atomically.
pub async fn append_thread_entry_with_attachment(
    pool: &PgPool,
    ticket_id: i64,
    entry: &NewThreadEntry,
    store: &BlobStore,
    attachment: &AttachmentSpec,
) -> Result<(ThreadEntry, AttachmentView), TicketError> {
    let exists: Option<(i64,)> = sqlx::query_as("SELECT ticket_id FROM ticket WHERE ticket_id = $1")
        .bind(ticket_id)
        .fetch_optional(pool)
        .await?;
    if exists.is_none() {
        return Err(TicketError::NotFound);
    }

    let mut tx = pool.begin().await?;

    let row = sqlx::query(
        "INSERT INTO ticket_thread (ticket_id, thread_type, poster, staff_id, body)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, ticket_id, thread_type, poster, body",
    )
    .bind(ticket_id)
    .bind(entry.thread_type.as_db())
    .bind(&entry.poster)
    .bind(entry.staff_id)
    .bind(&entry.body)
    .fetch_one(&mut *tx)
    .await?;
    let new_entry = ThreadEntry {
        id: row.get("id"),
        ticket_id: row.get("ticket_id"),
        thread_type: row.get("thread_type"),
        poster: row.get("poster"),
        body: row.get("body"),
    };

    // Store the blob, then bind it to the new entry — all before commit.
    store
        .put(&attachment.bytes)
        .await
        .map_err(|e| TicketError::Db(blob_io_to_sqlx(e)))?;
    let view = insert_attachment(
        &mut tx,
        ticket_id,
        new_entry.id,
        entry.thread_type.as_db(),
        attachment,
    )
    .await?;

    tx.commit().await?;
    Ok((new_entry, view))
}

/// Load a ticket's thread entries in chronological order (oldest first).
///
/// @implements BS-021: ordered thread retrieval (M then R …).
pub async fn load_thread(pool: &PgPool, ticket_id: i64) -> Result<Vec<ThreadEntry>, TicketError> {
    let rows = sqlx::query(
        "SELECT id, ticket_id, thread_type, poster, body FROM ticket_thread
         WHERE ticket_id = $1 ORDER BY id ASC",
    )
    .bind(ticket_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| ThreadEntry {
            id: row.get("id"),
            ticket_id: row.get("ticket_id"),
            thread_type: row.get("thread_type"),
            poster: row.get("poster"),
            body: row.get("body"),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_ticket_number_is_in_range() {
        for _ in 0..10_000 {
            let n = random_ticket_number();
            assert!(
                (TICKET_NUMBER_MIN..=TICKET_NUMBER_MAX).contains(&n),
                "{n} out of 6-digit range"
            );
        }
    }

    #[test]
    fn thread_type_db_literals() {
        assert_eq!(ThreadType::Message.as_db(), "M");
        assert_eq!(ThreadType::Response.as_db(), "R");
        assert_eq!(ThreadType::Note.as_db(), "N");
    }

    #[test]
    fn validated_rejects_missing_fields_with_field_map() {
        let err = NewTicket::validated(NewTicketInput::default()).unwrap_err();
        let fields: Vec<&str> = err.iter().map(|(f, _)| f.as_str()).collect();
        assert!(fields.contains(&"email"));
        assert!(fields.contains(&"name"));
        assert!(fields.contains(&"subject"));
        assert!(fields.contains(&"body"));
    }

    #[test]
    fn validated_sanitises_body_and_subject() {
        let input = NewTicketInput {
            email: "user@example.com".into(),
            name: "Jane".into(),
            subject: "<script>x</script>Help".into(),
            body: "<b>hi</b><script>evil()</script>".into(),
            ..Default::default()
        };
        let t = NewTicket::validated(input).unwrap();
        assert!(!t.subject.contains("<script>"), "subject stripped: {}", t.subject);
        assert!(!t.body.contains("script"), "body script removed: {}", t.body);
        assert_eq!(t.source, "Web", "default source");
    }

    #[test]
    fn unique_violation_detection() {
        // A non-DB error is not a unique violation.
        let other = sqlx::Error::RowNotFound;
        assert!(!is_unique_violation(&other));
    }
}
