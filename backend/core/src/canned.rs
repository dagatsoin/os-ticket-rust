//! Canned-response retrieval core (TS-M2-D2 / D4).
//!
//! The transport-agnostic queries the staff canned routes (and the D4 reply
//! wiring) reuse: list the offerable canned responses for a ticket, load one
//! with its (raw) body + attachments, and assemble the `%{...}` substitution
//! context from a ticket row. Body substitution itself is done by
//! [`crate::variable::VariableReplacer`] — this module supplies the data.
//!
//! Offer scope (BS-022.1 / BS-022.2): a canned response is offerable for a
//! ticket when it is **enabled** AND its `dept_id` is `0` (all departments) OR
//! equals the ticket's department. A disabled or out-of-scope id is not
//! fetchable (the route turns the `None` into a 404, no existence leak).
//!
//! @implements FS-022.14: canned-response retrieval (list + substituted detail).
//! @implements BS-022.1: dept scope — `dept_id = 0` (all) OR the ticket's dept.
//! @implements BS-022.2: only enabled responses are offerable / fetchable.

use serde::Serialize;
use sqlx::postgres::PgPool;
use sqlx::Row;

use crate::attachment::AttachmentView;
use crate::variable::VarContext;

/// One offerable canned response in the list route: `{ id, title }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CannedListItem {
    pub id: i32,
    pub title: String,
}

/// A canned response loaded for a specific ticket: its RAW body (the caller
/// substitutes it via [`crate::variable::VariableReplacer`]) plus its §7
/// attachment list and the bound `attachment_file` ids (the D4 reply wiring
/// binds those ids to the new `R` entry — no re-upload).
#[derive(Debug, Clone)]
pub struct CannedResponse {
    pub id: i32,
    pub title: String,
    /// The raw, un-substituted body (`response` column).
    pub body: String,
    /// The §7 attachment views `[{id, name, size, mime}]` — the `id` is the
    /// shared `attachment_file` id (canned attachments are not yet bound to a
    /// ticket thread entry, so there is no `ticket_attachment` binding id yet).
    pub attachments: Vec<AttachmentView>,
    /// The bound `attachment_file` ids (for the D4 reply carry — bind by id).
    pub file_ids: Vec<i64>,
}

/// The ticket fields needed to build the `%{ticket.*}` substitution context and
/// to scope which canned responses are offerable.
#[derive(Debug, Clone)]
pub struct TicketVars {
    pub dept_id: i32,
    pub number: String,
    pub name: String,
    pub subject: String,
    pub email: String,
    pub status: String,
    pub create_date: String,
    pub dept_name: String,
}

impl TicketVars {
    /// Build the `%{ticket.*}` + `%{ticket.dept.name}` context (FS-040.11) for
    /// this ticket, ready to pass to [`crate::variable::VariableReplacer::new`].
    #[must_use]
    pub fn to_context(&self) -> VarContext {
        VarContext::ticket(
            &self.number,
            &self.name,
            &self.subject,
            &self.email,
            &self.status,
            &self.create_date,
            &self.dept_name,
        )
    }
}

/// Load the ticket fields needed for canned scoping + substitution.
///
/// `None` when no ticket has that internal id (the route turns that into a 404).
///
/// @implements FS-040.11: assemble the `%{ticket.*}` catalog from a ticket row.
pub async fn load_ticket_vars(
    pool: &PgPool,
    ticket_id: i64,
) -> Result<Option<TicketVars>, sqlx::Error> {
    let row = sqlx::query(
        r#"SELECT t.dept_id, t."ticketID" AS number, t.name, t.subject, t.email, t.status,
                  to_char(t.created, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS create_date,
                  COALESCE(d.dept_name, '') AS dept_name
           FROM ticket t
           LEFT JOIN department d ON d.dept_id = t.dept_id
           WHERE t.ticket_id = $1"#,
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| {
        let number: i64 = r.get("number");
        TicketVars {
            dept_id: r.get("dept_id"),
            number: number.to_string(),
            name: r.get("name"),
            subject: r.get("subject"),
            email: r.get("email"),
            status: r.get("status"),
            create_date: r.get("create_date"),
            dept_name: r.get("dept_name"),
        }
    }))
}

/// List the canned responses offerable for a ticket in `dept_id`: enabled, and
/// scoped to `dept_id = 0` (all) OR the ticket's department, by `title` ASC.
///
/// @implements BS-022.1 / BS-022.2: enabled + dept-scoped offer list.
pub async fn list_offerable(
    pool: &PgPool,
    ticket_dept_id: i32,
) -> Result<Vec<CannedListItem>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT canned_id, title FROM canned_response
         WHERE isenabled = true AND (dept_id = 0 OR dept_id = $1)
         ORDER BY title ASC",
    )
    .bind(ticket_dept_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| CannedListItem {
            id: r.get("canned_id"),
            title: r.get("title"),
        })
        .collect())
}

/// Load a single canned response IF it is offerable for `ticket_dept_id`
/// (enabled + dept 0/all or the ticket's dept), with its attachments.
///
/// `None` when the id is unknown, disabled, or out of the ticket's dept scope —
/// the route turns that into a 404 (BS-022.2, no existence leak).
///
/// @implements FS-022.14 / BS-022.2: scoped canned detail + attachment list.
pub async fn load_offerable_response(
    pool: &PgPool,
    canned_id: i32,
    ticket_dept_id: i32,
) -> Result<Option<CannedResponse>, sqlx::Error> {
    let head = sqlx::query(
        "SELECT canned_id, title, response FROM canned_response
         WHERE canned_id = $1 AND isenabled = true AND (dept_id = 0 OR dept_id = $2)",
    )
    .bind(canned_id)
    .bind(ticket_dept_id)
    .fetch_optional(pool)
    .await?;

    let Some(head) = head else {
        return Ok(None);
    };
    let id: i32 = head.get("canned_id");
    let title: String = head.get("title");
    let body: String = head.get("response");

    // Attachments: the §7 view (id/name/size/mime) is keyed on the shared
    // attachment_file id, plus the bare file_ids for the D4 reply carry.
    let att_rows = sqlx::query(
        "SELECT af.id, af.name, af.size, af.mime
         FROM canned_attachment ca
         JOIN attachment_file af ON af.id = ca.file_id
         WHERE ca.canned_id = $1
         ORDER BY ca.id ASC",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    let mut attachments = Vec::with_capacity(att_rows.len());
    let mut file_ids = Vec::with_capacity(att_rows.len());
    for r in att_rows {
        let file_id: i64 = r.get("id");
        file_ids.push(file_id);
        attachments.push(AttachmentView {
            id: file_id,
            name: r.get("name"),
            size: r.get("size"),
            mime: r.get("mime"),
        });
    }

    Ok(Some(CannedResponse {
        id,
        title,
        body,
        attachments,
        file_ids,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variable::VariableReplacer;

    #[test]
    fn ticket_vars_build_substitution_context() {
        let vars = TicketVars {
            dept_id: 2,
            number: "123456".into(),
            name: "Mia".into(),
            subject: "Help".into(),
            email: "mia@example.com".into(),
            status: "open".into(),
            create_date: "2026-06-11T10:00:00Z".into(),
            dept_name: "Support".into(),
        };
        let r = VariableReplacer::new(vars.to_context(), "http://localhost:3702");
        assert_eq!(
            r.render("#%{ticket.number} (%{ticket.dept.name}) %{url}"),
            "#123456 (Support) http://localhost:3702"
        );
    }
}
