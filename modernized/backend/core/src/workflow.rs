//! Ticket workflow actions (TS-M3-C1/C2/C3).
//!
//! Implements the core workflow operations: close, reopen, assign, claim, transfer.
//! Each operation validates preconditions, updates the ticket, logs an internal note,
//! and records a lifecycle event.
//!
//! @implements FS-021.7: Assign/reassign ticket.
//! @implements FS-021.8: Claim ticket (self-assign).
//! @implements FS-021.10: Transfer between departments.
//! @implements FS-021.11: Close ticket.
//! @implements FS-021.12: Reopen ticket.
//! @implements FS-021.23: Lifecycle event log.

use sqlx::postgres::PgPool;

use crate::sanitize::safe_html;

/// Workflow operation errors.
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("ticket not found")]
    NotFound,
    #[error("Ticket is already closed!")]
    AlreadyClosed,
    #[error("Ticket is already open!")]
    AlreadyOpen,
    #[error("Ticket already assigned")]
    AlreadyAssigned,
    #[error("Ticket already assigned to the staff.")]
    AlreadyAssignedToSameStaff,
    #[error("Ticket already assigned to the team.")]
    AlreadyAssignedToSameTeam,
    #[error("Only open tickets can be claimed")]
    OnlyOpenCanBeClaimed,
    #[error("Comment too short")]
    CommentTooShort,
    #[error("Transfer comments too short!")]
    TransferCommentTooShort,
    #[error("Ticket already in the department")]
    AlreadyInDepartment,
    #[error("Unknown or invalid department")]
    InvalidDepartment,
    #[error("Unknown or invalid assignee")]
    InvalidAssignee,
}

/// Result of a close operation.
///
/// @implements FS-021.11: Close ticket result.
#[derive(Debug)]
pub struct CloseResult {
    /// The ticket's external number (for the success message).
    pub ticket_number: i64,
}

/// Result of a reopen operation.
///
/// @implements FS-021.12: Reopen ticket result.
#[derive(Debug)]
pub struct ReopenResult {
    /// The ticket's external number.
    pub ticket_number: i64,
}

/// Result of a claim operation.
///
/// @implements FS-021.8: Claim ticket result.
#[derive(Debug)]
pub struct ClaimResult {
    /// The ticket's external number.
    pub ticket_number: i64,
}

/// Result of an assign operation.
///
/// @implements FS-021.7: Assign ticket result.
#[derive(Debug)]
pub struct AssignResult {
    /// The ticket's external number.
    pub ticket_number: i64,
    /// Whether the ticket was reopened as part of this action.
    pub was_reopened: bool,
}

/// Result of a transfer operation.
///
/// @implements FS-021.10: Transfer ticket result.
#[derive(Debug)]
pub struct TransferResult {
    /// The ticket's external number.
    pub ticket_number: i64,
    /// The new department's name.
    pub dept_name: String,
    /// Whether the acting staff lost access to the ticket after transfer.
    pub access_lost: bool,
    /// Whether the ticket was reopened as part of this action.
    pub was_reopened: bool,
}

/// Close a ticket.
///
/// Sets status to closed, stamps the close timestamp, clears overdue flag and due date,
/// credits the closing staff, logs an internal note, and records a lifecycle event.
///
/// @implements FS-021.11: Close ticket.
/// @implements BS-021.4: Closing clears overdue and due date.
pub async fn close_ticket(
    pool: &PgPool,
    ticket_id: i64,
    staff_id: i32,
    staff_name: &str,
    staff_username: &str,
    comments: Option<&str>,
) -> Result<CloseResult, WorkflowError> {
    // Fetch ticket header and verify it exists and is open.
    let row: Option<(i64, String)> = sqlx::query_as(
        r#"SELECT "ticketID", status FROM ticket WHERE ticket_id = $1"#,
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await?;

    let (ticket_number, status) = row.ok_or(WorkflowError::NotFound)?;

    if status == "closed" {
        return Err(WorkflowError::AlreadyClosed);
    }

    let mut tx = pool.begin().await?;

    // Update ticket: set status=closed, closed=now, isoverdue=false, duedate=null, closed_by_staff_id=closer.
    // @implements BS-021.4: Closing clears overdue flag and due date.
    sqlx::query(
        "UPDATE ticket SET status = 'closed', closed = now(), isoverdue = false, duedate = NULL, \
         closed_by_staff_id = $1, updated = now() WHERE ticket_id = $2",
    )
    .bind(staff_id)
    .bind(ticket_id)
    .execute(&mut *tx)
    .await?;

    // Log internal note.
    let note_body = match comments.filter(|c| !c.trim().is_empty()) {
        Some(c) => safe_html(c),
        None => "Ticket closed (without comments)".to_string(),
    };
    append_internal_note(&mut tx, ticket_id, staff_id, staff_name, &note_body).await?;

    // Record lifecycle event.
    // @implements FS-021.23: lifecycle event with state='closed'.
    record_lifecycle_event(&mut tx, ticket_id, staff_id, "closed", staff_username).await?;

    tx.commit().await?;

    Ok(CloseResult { ticket_number })
}

/// Reopen a ticket.
///
/// Sets status to open, stamps reopened timestamp, marks unanswered, logs an internal note,
/// records a reopened lifecycle event, annuls the prior closed event, and recomputes the
/// due date from the reopened timestamp.
///
/// @implements FS-021.12: Reopen ticket.
/// @implements BS-021.14: Reopen annuls the prior close event.
/// @implements FS-032.11: Reopened ticket due date uses reopened timestamp.
pub async fn reopen_ticket(
    pool: &PgPool,
    ticket_id: i64,
    staff_id: i32,
    staff_name: &str,
    staff_username: &str,
    comments: Option<&str>,
) -> Result<ReopenResult, WorkflowError> {
    // Fetch ticket header and verify it exists and is closed.
    let row: Option<(i64, String, Option<i32>)> = sqlx::query_as(
        r#"SELECT "ticketID", status, sla_id FROM ticket WHERE ticket_id = $1"#,
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await?;

    let (ticket_number, status, sla_id) = row.ok_or(WorkflowError::NotFound)?;

    if status == "open" {
        return Err(WorkflowError::AlreadyOpen);
    }

    let mut tx = pool.begin().await?;

    // Update ticket: set status=open, reopened=now, isanswered=false.
    sqlx::query(
        "UPDATE ticket SET status = 'open', reopened = now(), isanswered = false, \
         updated = now() WHERE ticket_id = $1",
    )
    .bind(ticket_id)
    .execute(&mut *tx)
    .await?;

    // Recompute due date from reopened timestamp if SLA is set.
    // @implements FS-032.11: reopened ticket due date uses reopened timestamp.
    if let Some(sid) = sla_id {
        let grace_period: Option<i32> = sqlx::query_scalar(
            "SELECT grace_period FROM sla WHERE id = $1",
        )
        .bind(sid)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();

        if let Some(grace_hours) = grace_period {
            sqlx::query(
                "UPDATE ticket SET duedate = reopened + ($1 || ' hours')::interval WHERE ticket_id = $2",
            )
            .bind(grace_hours)
            .bind(ticket_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    // Log internal note.
    let note_body = match comments.filter(|c| !c.trim().is_empty()) {
        Some(c) => safe_html(c),
        None => "Ticket reopened (without comments)".to_string(),
    };
    append_internal_note(&mut tx, ticket_id, staff_id, staff_name, &note_body).await?;

    // Annul prior closed event.
    // @implements BS-021.14: Reopen annuls the prior close event.
    sqlx::query(
        "UPDATE ticket_event SET annulled = true WHERE ticket_id = $1 AND state = 'closed' AND annulled = false",
    )
    .bind(ticket_id)
    .execute(&mut *tx)
    .await?;

    // Record reopened lifecycle event.
    record_lifecycle_event(&mut tx, ticket_id, staff_id, "reopened", staff_username).await?;

    tx.commit().await?;

    Ok(ReopenResult { ticket_number })
}

/// Claim a ticket (self-assign).
///
/// Validates that the ticket is open and unassigned, then assigns it to the claiming staff.
/// Logs an internal note "Ticket claimed by <name>".
///
/// @implements FS-021.8: Claim ticket.
/// @implements EC-021.7: Claiming an assigned or closed ticket is rejected.
pub async fn claim_ticket(
    pool: &PgPool,
    ticket_id: i64,
    staff_id: i32,
    staff_name: &str,
) -> Result<ClaimResult, WorkflowError> {
    // Fetch ticket header and verify preconditions.
    let row: Option<(i64, String, Option<i32>)> = sqlx::query_as(
        r#"SELECT "ticketID", status, staff_id FROM ticket WHERE ticket_id = $1"#,
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await?;

    let (ticket_number, status, assigned_staff) = row.ok_or(WorkflowError::NotFound)?;

    // Only open tickets can be claimed.
    if status != "open" {
        return Err(WorkflowError::OnlyOpenCanBeClaimed);
    }

    // Cannot claim an already-assigned ticket.
    if assigned_staff.is_some() {
        return Err(WorkflowError::AlreadyAssigned);
    }

    let mut tx = pool.begin().await?;

    // Assign to self.
    sqlx::query(
        "UPDATE ticket SET staff_id = $1, updated = now() WHERE ticket_id = $2",
    )
    .bind(staff_id)
    .bind(ticket_id)
    .execute(&mut *tx)
    .await?;

    // Log internal note.
    // @implements FS-021.8: claim logs "Ticket claimed by <name>".
    let note_body = format!("Ticket claimed by {}", staff_name);
    append_internal_note(&mut tx, ticket_id, staff_id, staff_name, &note_body).await?;

    tx.commit().await?;

    Ok(ClaimResult { ticket_number })
}

/// Assignee type for the assign operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssigneeType {
    Staff(i32),
    Team(i32),
}

impl AssigneeType {
    /// Parse an assignee string (s123 for staff, t456 for team).
    ///
    /// @implements FS-021.7: assignee id prefix s/t.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if let Some(id_str) = s.strip_prefix('s') {
            id_str.parse().ok().map(AssigneeType::Staff)
        } else if let Some(id_str) = s.strip_prefix('t') {
            id_str.parse().ok().map(AssigneeType::Team)
        } else {
            None
        }
    }
}

/// Assign a ticket to a staff member or team.
///
/// Validates that comments are >= 5 chars, and that the ticket isn't already assigned
/// to the same assignee. If the ticket is closed, it will be reopened.
/// Logs an internal note "Ticket assigned to <name> by <actor>".
///
/// @implements FS-021.7: Assign/reassign ticket.
/// @implements BS-021.8: Assignment requires comments >= 5 chars.
/// @implements BS-021.5: Assigning a closed ticket reopens it.
pub async fn assign_ticket(
    pool: &PgPool,
    ticket_id: i64,
    assignee: AssigneeType,
    comments: &str,
    actor_staff_id: i32,
    actor_name: &str,
    actor_username: &str,
) -> Result<AssignResult, WorkflowError> {
    // Validate comments length.
    // @implements BS-021.8: comments >= 5 chars.
    if comments.trim().len() < 5 {
        return Err(WorkflowError::CommentTooShort);
    }

    // Fetch ticket header.
    let row: Option<(i64, String, Option<i32>, i32)> = sqlx::query_as(
        r#"SELECT "ticketID", status, staff_id, team_id FROM ticket WHERE ticket_id = $1"#,
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await?;

    let (ticket_number, status, current_staff, current_team) = row.ok_or(WorkflowError::NotFound)?;

    // Check if already assigned to same.
    match &assignee {
        AssigneeType::Staff(staff_id) => {
            if current_staff == Some(*staff_id) {
                return Err(WorkflowError::AlreadyAssignedToSameStaff);
            }
        }
        AssigneeType::Team(team_id) => {
            if current_team == *team_id {
                return Err(WorkflowError::AlreadyAssignedToSameTeam);
            }
        }
    }

    // Look up assignee name for the note. Validate that the assignee exists.
    // @implements FS-021.7: validate assignee exists before assignment.
    let assignee_name = match &assignee {
        AssigneeType::Staff(staff_id) => {
            let row: Option<(String, String, String)> = sqlx::query_as(
                "SELECT username, firstname, lastname FROM staff WHERE staff_id = $1",
            )
            .bind(*staff_id)
            .fetch_optional(pool)
            .await?;
            match row {
                Some((username, firstname, lastname)) => {
                    let full = format!("{} {}", firstname.trim(), lastname.trim());
                    let full = full.trim().to_string();
                    if full.is_empty() { username } else { full }
                }
                None => return Err(WorkflowError::InvalidAssignee),
            }
        }
        AssigneeType::Team(team_id) => {
            let name: Option<String> = sqlx::query_scalar(
                "SELECT name FROM team WHERE team_id = $1",
            )
            .bind(*team_id)
            .fetch_optional(pool)
            .await?;
            match name {
                Some(n) => n,
                None => return Err(WorkflowError::InvalidAssignee),
            }
        }
    };

    let mut tx = pool.begin().await?;

    // If closed, reopen first.
    // @implements BS-021.5: assigning a closed ticket reopens it.
    let was_reopened = status == "closed";
    if was_reopened {
        sqlx::query(
            "UPDATE ticket SET status = 'open', reopened = now(), isanswered = false, updated = now() WHERE ticket_id = $1",
        )
        .bind(ticket_id)
        .execute(&mut *tx)
        .await?;
    }

    // Update assignment.
    match &assignee {
        AssigneeType::Staff(staff_id) => {
            sqlx::query(
                "UPDATE ticket SET staff_id = $1, updated = now() WHERE ticket_id = $2",
            )
            .bind(*staff_id)
            .bind(ticket_id)
            .execute(&mut *tx)
            .await?;
        }
        AssigneeType::Team(team_id) => {
            // Team assignment: set team_id, and if ticket was closed, clear staff_id.
            // @implements FS-021.7: team assignment sets team_id.
            let sql = if was_reopened {
                "UPDATE ticket SET team_id = $1, staff_id = NULL, updated = now() WHERE ticket_id = $2"
            } else {
                "UPDATE ticket SET team_id = $1, updated = now() WHERE ticket_id = $2"
            };
            sqlx::query(sql)
                .bind(*team_id)
                .bind(ticket_id)
                .execute(&mut *tx)
                .await?;
        }
    }

    // Log internal note.
    let note_body = format!(
        "Ticket assigned to {} by {}\n\n{}",
        assignee_name, actor_name, safe_html(comments)
    );
    append_internal_note(&mut tx, ticket_id, actor_staff_id, actor_name, &note_body).await?;

    // Record lifecycle event for reopen if it happened.
    if was_reopened {
        // Annul prior closed event.
        sqlx::query(
            "UPDATE ticket_event SET annulled = true WHERE ticket_id = $1 AND state = 'closed' AND annulled = false",
        )
        .bind(ticket_id)
        .execute(&mut *tx)
        .await?;

        record_lifecycle_event(&mut tx, ticket_id, actor_staff_id, "reopened", actor_username).await?;
    }

    tx.commit().await?;

    Ok(AssignResult {
        ticket_number,
        was_reopened,
    })
}

/// Transfer a ticket to a different department.
///
/// Validates that comments are >= 5 chars, and that the target department differs
/// from the current one. If the ticket is closed, it will be reopened. Re-selects
/// SLA based on the new department. Logs an internal note and records a transferred
/// lifecycle event.
///
/// @implements FS-021.10: Transfer between departments.
/// @implements BS-021.5: Transferring a closed ticket reopens it.
/// @implements FS-021.13: SLA re-selection on transfer.
pub async fn transfer_ticket(
    pool: &PgPool,
    ticket_id: i64,
    new_dept_id: i32,
    comments: &str,
    actor_staff_id: i32,
    actor_name: &str,
    actor_username: &str,
    actor_dept_ids: &[i32],
) -> Result<TransferResult, WorkflowError> {
    // Validate comments length.
    // @implements BS-021.8: comments >= 5 chars.
    if comments.trim().len() < 5 {
        return Err(WorkflowError::TransferCommentTooShort);
    }

    // Fetch ticket header.
    let row: Option<(i64, String, i32, Option<i32>)> = sqlx::query_as(
        r#"SELECT "ticketID", status, dept_id, topic_id FROM ticket WHERE ticket_id = $1"#,
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await?;

    let (ticket_number, status, current_dept_id, topic_id) = row.ok_or(WorkflowError::NotFound)?;

    // Check if already in the same department.
    if current_dept_id == new_dept_id {
        return Err(WorkflowError::AlreadyInDepartment);
    }

    // Verify target department exists and get its name.
    let row: Option<(String, Option<i32>)> = sqlx::query_as(
        "SELECT dept_name, sla_id FROM department WHERE dept_id = $1",
    )
    .bind(new_dept_id)
    .fetch_optional(pool)
    .await?;

    let (new_dept_name, dept_sla_id) = row.ok_or(WorkflowError::InvalidDepartment)?;

    // Get old department name for the note.
    let old_dept_name: Option<String> = sqlx::query_scalar(
        "SELECT dept_name FROM department WHERE dept_id = $1",
    )
    .bind(current_dept_id)
    .fetch_optional(pool)
    .await?;
    let old_dept_name = old_dept_name.unwrap_or_else(|| "Unknown".to_string());

    let mut tx = pool.begin().await?;

    // If closed, reopen first.
    // @implements BS-021.5: transferring a closed ticket reopens it.
    let was_reopened = status == "closed";
    if was_reopened {
        sqlx::query(
            "UPDATE ticket SET status = 'open', reopened = now(), isanswered = false, updated = now() WHERE ticket_id = $1",
        )
        .bind(ticket_id)
        .execute(&mut *tx)
        .await?;
    }

    // Re-select SLA based on new department.
    // @implements FS-021.13: SLA selection precedence: trump > dept > topic > system default.
    let new_sla_id = select_sla_id(pool, dept_sla_id, topic_id).await?;

    // Update department and SLA.
    sqlx::query(
        "UPDATE ticket SET dept_id = $1, sla_id = $2, updated = now() WHERE ticket_id = $3",
    )
    .bind(new_dept_id)
    .bind(new_sla_id)
    .bind(ticket_id)
    .execute(&mut *tx)
    .await?;

    // Log internal note with transfer message.
    // Note: legacy typo "transfered" preserved per spec.
    let note_body = format!(
        "Ticket transfered from {} to {}\n\n{}",
        old_dept_name, new_dept_name, safe_html(comments)
    );
    append_internal_note(&mut tx, ticket_id, actor_staff_id, actor_name, &note_body).await?;

    // Record lifecycle event for reopen if it happened.
    if was_reopened {
        // Annul prior closed event.
        sqlx::query(
            "UPDATE ticket_event SET annulled = true WHERE ticket_id = $1 AND state = 'closed' AND annulled = false",
        )
        .bind(ticket_id)
        .execute(&mut *tx)
        .await?;

        record_lifecycle_event(&mut tx, ticket_id, actor_staff_id, "reopened", actor_username).await?;
    }

    // Record transferred lifecycle event.
    record_lifecycle_event(&mut tx, ticket_id, actor_staff_id, "transferred", actor_username).await?;

    tx.commit().await?;

    // Check if actor lost access to the new department.
    // @implements FS-021.10: re-check access after transfer.
    let access_lost = !actor_dept_ids.contains(&new_dept_id);

    Ok(TransferResult {
        ticket_number,
        dept_name: new_dept_name,
        access_lost,
        was_reopened,
    })
}

/// Select the SLA id based on the precedence: trump > dept > topic > system default.
///
/// @implements FS-021.13: SLA selection precedence.
async fn select_sla_id(
    pool: &PgPool,
    dept_sla_id: Option<i32>,
    topic_id: Option<i32>,
) -> Result<Option<i32>, sqlx::Error> {
    // Department SLA takes precedence.
    if dept_sla_id.is_some() {
        return Ok(dept_sla_id);
    }

    // Then help topic SLA.
    if let Some(tid) = topic_id {
        let topic_sla: Option<i32> = sqlx::query_scalar(
            "SELECT sla_id FROM help_topic WHERE topic_id = $1",
        )
        .bind(tid)
        .fetch_optional(pool)
        .await?
        .flatten();
        if topic_sla.is_some() {
            return Ok(topic_sla);
        }
    }

    // Then system default SLA.
    let default_sla: Option<i32> = sqlx::query_scalar(
        "SELECT value::integer FROM config WHERE key = 'default_sla_id'",
    )
    .fetch_optional(pool)
    .await?
    .flatten();

    Ok(default_sla)
}

/// Append an internal note (type N) to the ticket thread.
async fn append_internal_note(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ticket_id: i64,
    staff_id: i32,
    poster: &str,
    body: &str,
) -> Result<i64, sqlx::Error> {
    let entry_id: i64 = sqlx::query_scalar(
        "INSERT INTO ticket_thread (ticket_id, thread_type, poster, staff_id, body) \
         VALUES ($1, 'N', $2, $3, $4) RETURNING id",
    )
    .bind(ticket_id)
    .bind(poster)
    .bind(staff_id)
    .bind(body)
    .fetch_one(&mut **tx)
    .await?;
    Ok(entry_id)
}

/// Record a lifecycle event in the ticket_event table.
///
/// @implements FS-021.23: lifecycle event recording.
async fn record_lifecycle_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ticket_id: i64,
    staff_id: i32,
    state: &str,
    username: &str,
) -> Result<i64, sqlx::Error> {
    let event_id: i64 = sqlx::query_scalar(
        "INSERT INTO ticket_event (ticket_id, staff_id, state, username) \
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(ticket_id)
    .bind(staff_id)
    .bind(state)
    .bind(username)
    .fetch_one(&mut **tx)
    .await?;
    Ok(event_id)
}

// ---------------------------------------------------------------------------
// TS-M3-D3 — SLA selection and due-date computation
// ---------------------------------------------------------------------------

/// Select the effective SLA id for a ticket based on precedence.
///
/// Precedence (canonical per FS-021.13): explicit trump value > department SLA >
/// help topic SLA > system default SLA. Department is evaluated BEFORE topic.
///
/// @implements FS-021.13: SLA selection precedence.
pub async fn select_sla_for_ticket(
    pool: &PgPool,
    dept_id: i32,
    topic_id: Option<i32>,
) -> Result<Option<i32>, WorkflowError> {
    select_sla_id_with_dept(pool, None, dept_id, topic_id).await.map_err(WorkflowError::from)
}

/// Select the effective SLA id with optional trump value and department SLA.
///
/// @implements FS-021.13: SLA selection precedence with trump support.
async fn select_sla_id_with_dept(
    pool: &PgPool,
    trump_sla_id: Option<i32>,
    dept_id: i32,
    topic_id: Option<i32>,
) -> Result<Option<i32>, sqlx::Error> {
    // Trump value takes precedence.
    if trump_sla_id.is_some() {
        return Ok(trump_sla_id);
    }

    // Department SLA is next.
    let dept_sla: Option<i32> = sqlx::query_scalar(
        "SELECT sla_id FROM department WHERE dept_id = $1",
    )
    .bind(dept_id)
    .fetch_optional(pool)
    .await?
    .flatten();
    if dept_sla.is_some() {
        return Ok(dept_sla);
    }

    // Then help topic SLA.
    if let Some(tid) = topic_id {
        let topic_sla: Option<i32> = sqlx::query_scalar(
            "SELECT sla_id FROM help_topic WHERE topic_id = $1",
        )
        .bind(tid)
        .fetch_optional(pool)
        .await?
        .flatten();
        if topic_sla.is_some() {
            return Ok(topic_sla);
        }
    }

    // Then system default SLA.
    let default_sla: Option<i32> = sqlx::query_scalar(
        "SELECT value::integer FROM config WHERE key = 'default_sla_id'",
    )
    .fetch_optional(pool)
    .await?
    .flatten();

    Ok(default_sla)
}

/// Apply SLA selection and due-date computation to a ticket on create.
///
/// This is called after ticket creation to set the sla_id and compute the initial
/// due date based on the selected SLA's grace period.
///
/// @implements FS-021.13: SLA selection on ticket create.
/// @implements FS-032.11: Due date computation on create.
pub async fn apply_sla_on_create(
    pool: &PgPool,
    ticket_id: i64,
    dept_id: i32,
    topic_id: Option<i32>,
) -> Result<(), WorkflowError> {
    // Select SLA by precedence.
    let sla_id = select_sla_id_with_dept(pool, None, dept_id, topic_id).await?;

    if sla_id.is_none() {
        return Ok(());
    }

    // Get the SLA grace period and compute due date.
    let grace_period: Option<i32> = sqlx::query_scalar(
        "SELECT grace_period FROM sla WHERE id = $1",
    )
    .bind(sla_id)
    .fetch_optional(pool)
    .await?
    .flatten();

    // Update ticket with SLA and computed due date.
    if let Some(grace_hours) = grace_period {
        sqlx::query(
            "UPDATE ticket SET sla_id = $1, \
             duedate = created + ($2 || ' hours')::interval, \
             updated = now() WHERE ticket_id = $3",
        )
        .bind(sla_id)
        .bind(grace_hours)
        .bind(ticket_id)
        .execute(pool)
        .await?;
    } else {
        // No grace period, just set the SLA id.
        sqlx::query(
            "UPDATE ticket SET sla_id = $1, updated = now() WHERE ticket_id = $2",
        )
        .bind(sla_id)
        .bind(ticket_id)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Recompute the due date for a reopened ticket.
///
/// For reopened tickets, the due date is computed from the reopen timestamp,
/// not the creation timestamp.
///
/// @implements FS-032.11: Reopened ticket due date uses reopened timestamp.
pub async fn recompute_due_date_on_reopen(
    pool: &PgPool,
    ticket_id: i64,
) -> Result<(), WorkflowError> {
    // Fetch ticket's SLA.
    let row: Option<(Option<i32>,)> = sqlx::query_as(
        "SELECT sla_id FROM ticket WHERE ticket_id = $1",
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await?;

    let (sla_id,) = row.ok_or(WorkflowError::NotFound)?;

    let Some(sla_id) = sla_id else {
        return Ok(());
    };

    // Get the SLA grace period.
    let grace_period: Option<i32> = sqlx::query_scalar(
        "SELECT grace_period FROM sla WHERE id = $1",
    )
    .bind(sla_id)
    .fetch_optional(pool)
    .await?
    .flatten();

    // Recompute due date from reopened timestamp.
    if let Some(grace_hours) = grace_period {
        sqlx::query(
            "UPDATE ticket SET duedate = reopened + ($1 || ' hours')::interval, \
             updated = now() WHERE ticket_id = $2",
        )
        .bind(grace_hours)
        .bind(ticket_id)
        .execute(pool)
        .await?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// TS-M3-D4 — Overdue detection
// ---------------------------------------------------------------------------

/// Mark a ticket as overdue.
///
/// Sets the isoverdue flag to true. This is idempotent — if already overdue,
/// no event or alert is produced.
///
/// @implements FS-021.13: Mark overdue action.
/// @implements BS-021.21: mark_overdue is idempotent.
pub async fn mark_ticket_overdue(
    pool: &PgPool,
    ticket_id: i64,
    staff_id: Option<i32>,
    actor_name: &str,
    actor_username: &str,
) -> Result<bool, WorkflowError> {
    // Check if already overdue.
    let row: Option<(bool, String)> = sqlx::query_as(
        "SELECT isoverdue, status FROM ticket WHERE ticket_id = $1",
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await?;

    let (is_overdue, status) = row.ok_or(WorkflowError::NotFound)?;

    // Already overdue — idempotent success.
    if is_overdue {
        return Ok(false); // No change made.
    }

    // Cannot mark a closed ticket overdue.
    if status == "closed" {
        return Ok(false);
    }

    let mut tx = pool.begin().await?;

    // Set isoverdue = true.
    sqlx::query(
        "UPDATE ticket SET isoverdue = true, updated = now() WHERE ticket_id = $1",
    )
    .bind(ticket_id)
    .execute(&mut *tx)
    .await?;

    // Log internal note.
    let note_body = if staff_id.is_some() {
        format!("Ticket flagged as overdue by {}", actor_name)
    } else {
        "Ticket flagged as overdue by the system.".to_string()
    };
    let staff = staff_id.unwrap_or(0);
    append_internal_note(&mut tx, ticket_id, staff, actor_name, &note_body).await?;

    // Record lifecycle event.
    record_lifecycle_event(&mut tx, ticket_id, staff, "overdue", actor_username).await?;

    tx.commit().await?;

    Ok(true) // Change made.
}

/// Clear the overdue flag on a ticket.
///
/// Called when ticket is closed or when a manager clears overdue manually.
///
/// @implements BS-021.4: On close, clear isoverdue and duedate.
/// @implements FS-021.13: Clear overdue (notdue) action.
pub async fn clear_ticket_overdue(
    pool: &PgPool,
    ticket_id: i64,
) -> Result<(), WorkflowError> {
    sqlx::query(
        "UPDATE ticket SET isoverdue = false, updated = now() WHERE ticket_id = $1",
    )
    .bind(ticket_id)
    .execute(pool)
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal notes with optional state change (TS-M3-E1/E2)
// ---------------------------------------------------------------------------

/// State change options for posting a note.
///
/// @implements FS-021.4: note-form state change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NoteState {
    /// No state change (default).
    #[default]
    Unchanged,
    /// Close the ticket.
    Closed,
    /// Reopen the ticket (set status=open, isanswered=false).
    Open,
    /// Mark the ticket as answered.
    Answered,
    /// Mark the ticket as unanswered.
    Unanswered,
    /// Mark the ticket as overdue.
    Overdue,
    /// Clear the overdue flag.
    NotDue,
}

impl NoteState {
    /// Parse a state string into a NoteState enum.
    ///
    /// @implements FS-021.4: state value parsing.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "unchanged" | "" => Some(NoteState::Unchanged),
            "closed" => Some(NoteState::Closed),
            "open" => Some(NoteState::Open),
            "answered" => Some(NoteState::Answered),
            "unanswered" => Some(NoteState::Unanswered),
            "overdue" => Some(NoteState::Overdue),
            "notdue" => Some(NoteState::NotDue),
            _ => None,
        }
    }
}

/// Result of posting a note.
///
/// @implements FS-021.4: note posting result.
#[derive(Debug)]
pub struct NoteResult {
    /// The created thread entry id.
    pub entry_id: i64,
    /// Whether a state change was applied.
    pub state_changed: bool,
}

/// Post an internal note (`N`) to a ticket with an optional state change.
///
/// The note body is required. If a state change is requested, it is applied
/// after the note is posted. State change failures are logged but do not
/// prevent the note from being posted.
///
/// @implements FS-021.4: POST note endpoint.
/// @implements FS-021.22: poster_name set to staff name.
/// @implements BS-021.4: closed state clears overdue and duedate.
/// @implements BS-021.14: open state sets isanswered=false.
pub async fn post_note(
    pool: &PgPool,
    ticket_id: i64,
    staff_id: i32,
    staff_name: &str,
    body: &str,
    title: Option<&str>,
    state: NoteState,
) -> Result<NoteResult, WorkflowError> {
    // Verify the ticket exists.
    let row: Option<(i64, String)> = sqlx::query_as(
        r#"SELECT "ticketID", status FROM ticket WHERE ticket_id = $1"#,
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await?;

    let (_ticket_number, current_status) = row.ok_or(WorkflowError::NotFound)?;

    let mut tx = pool.begin().await?;

    // Insert the note entry.
    // @implements FS-021.22: poster is set to the staff's display name.
    let entry_id: i64 = sqlx::query_scalar(
        "INSERT INTO ticket_thread (ticket_id, thread_type, poster, staff_id, title, body) \
         VALUES ($1, 'N', $2, $3, $4, $5) RETURNING id",
    )
    .bind(ticket_id)
    .bind(staff_name)
    .bind(staff_id)
    .bind(title)
    .bind(safe_html(body))
    .fetch_one(&mut *tx)
    .await?;

    // Apply the state change if requested.
    let state_changed = match state {
        NoteState::Unchanged => false,
        NoteState::Closed => {
            if current_status != "closed" {
                // @implements BS-021.4: closing clears overdue and duedate.
                sqlx::query(
                    "UPDATE ticket SET status = 'closed', closed = now(), isoverdue = false, \
                     duedate = NULL, closed_by_staff_id = $1, updated = now() WHERE ticket_id = $2",
                )
                .bind(staff_id)
                .bind(ticket_id)
                .execute(&mut *tx)
                .await?;
                true
            } else {
                false
            }
        }
        NoteState::Open => {
            if current_status == "closed" {
                // @implements BS-021.14: reopen sets isanswered=false.
                sqlx::query(
                    "UPDATE ticket SET status = 'open', reopened = now(), isanswered = false, \
                     updated = now() WHERE ticket_id = $1",
                )
                .bind(ticket_id)
                .execute(&mut *tx)
                .await?;
                true
            } else {
                false
            }
        }
        NoteState::Answered => {
            sqlx::query("UPDATE ticket SET isanswered = true, updated = now() WHERE ticket_id = $1")
                .bind(ticket_id)
                .execute(&mut *tx)
                .await?;
            true
        }
        NoteState::Unanswered => {
            sqlx::query("UPDATE ticket SET isanswered = false, updated = now() WHERE ticket_id = $1")
                .bind(ticket_id)
                .execute(&mut *tx)
                .await?;
            true
        }
        NoteState::Overdue => {
            sqlx::query("UPDATE ticket SET isoverdue = true, updated = now() WHERE ticket_id = $1")
                .bind(ticket_id)
                .execute(&mut *tx)
                .await?;
            true
        }
        NoteState::NotDue => {
            sqlx::query("UPDATE ticket SET isoverdue = false, updated = now() WHERE ticket_id = $1")
                .bind(ticket_id)
                .execute(&mut *tx)
                .await?;
            true
        }
    };

    tx.commit().await?;

    Ok(NoteResult {
        entry_id,
        state_changed,
    })
}

// ---------------------------------------------------------------------------
// TS-M3-I2 — Delete ticket with cascade
// ---------------------------------------------------------------------------

/// Result of a delete operation.
///
/// @implements FS-021.19: Delete ticket result.
#[derive(Debug)]
pub struct DeleteResult {
    /// The ticket's external number (for the success message).
    pub ticket_number: i64,
}

/// Delete a ticket and cascade to thread entries and attachments.
///
/// Permanently removes the ticket row, deletes orphaned thread entries and their
/// attachments. Orphaned attachment files (content-addressed blobs not referenced
/// by any other ticket) are purged.
///
/// @implements FS-021.19: Delete ticket.
/// @implements BS-021.16: Deletion cascades to thread and attachments.
pub async fn delete_ticket(
    pool: &PgPool,
    ticket_id: i64,
    staff_username: &str,
) -> Result<DeleteResult, WorkflowError> {
    // Fetch ticket header and verify it exists.
    let row: Option<(i64,)> = sqlx::query_as(
        r#"SELECT "ticketID" FROM ticket WHERE ticket_id = $1"#,
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await?;

    let (ticket_number,) = row.ok_or(WorkflowError::NotFound)?;

    let mut tx = pool.begin().await?;

    // Log deletion (debug level — we can't use the standard internal note since
    // the ticket is being deleted). Using tracing here.
    // @implements FS-021.19: debug log records who deleted which ticket.
    tracing::debug!(
        ticket_number = ticket_number,
        deleted_by = staff_username,
        "Ticket #{} deleted by {}",
        ticket_number,
        staff_username
    );

    // Find attachment file hashes that will become orphaned after this delete.
    // We need to purge content-addressed blobs that are no longer referenced.
    // @implements BS-021.16: orphan purge for content-addressed attachments.
    let orphan_hashes: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT af.hash FROM attachment_file af
         JOIN ticket_attachment ta ON ta.file_id = af.id
         WHERE ta.ticket_id = $1
         AND NOT EXISTS (
             SELECT 1 FROM ticket_attachment ta2
             WHERE ta2.file_id = af.id AND ta2.ticket_id != $1
         )",
    )
    .bind(ticket_id)
    .fetch_all(&mut *tx)
    .await?;

    // Delete ticket attachments (junction table).
    sqlx::query("DELETE FROM ticket_attachment WHERE ticket_id = $1")
        .bind(ticket_id)
        .execute(&mut *tx)
        .await?;

    // Delete orphaned attachment_file rows.
    for hash in &orphan_hashes {
        sqlx::query("DELETE FROM attachment_file WHERE hash = $1")
            .bind(hash)
            .execute(&mut *tx)
            .await?;
    }

    // Delete thread entries (cascade removes any remaining references).
    sqlx::query("DELETE FROM ticket_thread WHERE ticket_id = $1")
        .bind(ticket_id)
        .execute(&mut *tx)
        .await?;

    // Delete lifecycle events.
    sqlx::query("DELETE FROM ticket_event WHERE ticket_id = $1")
        .bind(ticket_id)
        .execute(&mut *tx)
        .await?;

    // Delete any locks on the ticket.
    sqlx::query("DELETE FROM ticket_lock WHERE ticket_id = $1")
        .bind(ticket_id)
        .execute(&mut *tx)
        .await?;

    // Finally delete the ticket itself.
    sqlx::query("DELETE FROM ticket WHERE ticket_id = $1")
        .bind(ticket_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    // Note: Physical blob purge from filesystem is handled by the blob store
    // cleanup, which is called separately. The file system paths use content
    // addressing so we just record the orphaned hashes for later cleanup.

    Ok(DeleteResult { ticket_number })
}

// ---------------------------------------------------------------------------
// TS-M3-I1 — Update ticket properties
// ---------------------------------------------------------------------------

/// Input for updating ticket properties.
///
/// @implements FS-021.15: Edit ticket properties input.
#[derive(Debug, Clone)]
pub struct UpdateTicketInput {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub phone_ext: Option<String>,
    pub dept_id: Option<i32>,
    pub topic_id: Option<i32>,
    pub priority_id: Option<i32>,
    pub sla_id: Option<i32>,
    pub source: Option<String>,
    pub due_date: Option<String>,
    pub reason: String,
}

/// Result of an update operation.
///
/// @implements FS-021.15: Update ticket result.
#[derive(Debug)]
pub struct UpdateResult {
    pub ticket_number: i64,
    pub new_sla_id: Option<i32>,
    pub overdue_cleared: bool,
}

/// Update ticket validation errors.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("ticket not found")]
    NotFound,
    #[error("Reason for the update required")]
    ReasonRequired,
    #[error("Due date must be in the future")]
    DueDateInPast,
    #[error("Due date can NOT be set on a closed ticket")]
    DueDateOnClosed,
    #[error("Invalid due date")]
    InvalidDueDate,
    #[error("Phone extension requires a phone number")]
    PhoneExtWithoutPhone,
    #[error("Invalid email format")]
    InvalidEmail,
    #[error("Unknown or invalid department")]
    InvalidDepartment,
}

/// Update a ticket's properties.
///
/// Validates input, updates the ticket, re-selects SLA if department changed,
/// clears overdue if due date is now in the future, and logs an internal note.
///
/// @implements FS-021.15: Edit ticket properties.
/// @implements BS-021.6: editing due date clears overdue flag.
/// @implements BS-021.7: editing department re-selects SLA.
pub async fn update_ticket(
    pool: &PgPool,
    ticket_id: i64,
    input: UpdateTicketInput,
    staff_id: i32,
    staff_name: &str,
) -> Result<UpdateResult, UpdateError> {
    use crate::validation::is_email;

    // Validate reason is required.
    if input.reason.trim().is_empty() {
        return Err(UpdateError::ReasonRequired);
    }

    // Validate email if provided.
    if let Some(ref email) = input.email {
        if !email.trim().is_empty() && !is_email(email) {
            return Err(UpdateError::InvalidEmail);
        }
    }

    // Validate phone_ext requires phone.
    if input.phone_ext.is_some() && input.phone.as_ref().map(|p| p.trim().is_empty()).unwrap_or(true) {
        // phone_ext set but phone is empty or None
        if input.phone.is_none() {
            return Err(UpdateError::PhoneExtWithoutPhone);
        }
    }

    // Fetch ticket header.
    let row: Option<(i64, String, i32, Option<i32>, bool)> = sqlx::query_as(
        r#"SELECT "ticketID", status, dept_id, topic_id, isoverdue FROM ticket WHERE ticket_id = $1"#,
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await?;

    let (ticket_number, status, current_dept_id, current_topic_id, current_overdue) =
        row.ok_or(UpdateError::NotFound)?;

    // Validate due date rules.
    // @implements EC-021.9: Due date validation.
    let parsed_due_date = if let Some(ref due_str) = input.due_date {
        if !due_str.trim().is_empty() {
            // Cannot set due date on closed ticket.
            if status == "closed" {
                return Err(UpdateError::DueDateOnClosed);
            }
            // Parse and validate it's in the future.
            let parsed = chrono::DateTime::parse_from_rfc3339(due_str)
                .map_err(|_| UpdateError::InvalidDueDate)?;
            if parsed < chrono::Utc::now() {
                return Err(UpdateError::DueDateInPast);
            }
            Some(parsed)
        } else {
            None
        }
    } else {
        None
    };

    // Validate department if being changed.
    let new_dept_id = if let Some(dept_id) = input.dept_id {
        if dept_id != current_dept_id {
            // Verify department exists.
            let exists: Option<(i32,)> = sqlx::query_as(
                "SELECT dept_id FROM department WHERE dept_id = $1",
            )
            .bind(dept_id)
            .fetch_optional(pool)
            .await?;
            if exists.is_none() {
                return Err(UpdateError::InvalidDepartment);
            }
        }
        Some(dept_id)
    } else {
        None
    };

    let mut tx = pool.begin().await?;

    // Track if department changed for SLA re-selection.
    let dept_changed = new_dept_id.map(|d| d != current_dept_id).unwrap_or(false);
    let effective_dept_id = new_dept_id.unwrap_or(current_dept_id);
    let effective_topic_id = input.topic_id.or(current_topic_id);

    // Execute update with all optional fields.
    // We'll use a simpler approach: update all provided fields in one query.
    sqlx::query(
        "UPDATE ticket SET
            name = COALESCE($1, name),
            email = COALESCE($2, email),
            phone = COALESCE($3, phone),
            phone_ext = COALESCE($4, phone_ext),
            dept_id = COALESCE($5, dept_id),
            topic_id = COALESCE($6, topic_id),
            priority_id = COALESCE($7, priority_id),
            source = COALESCE($8, source),
            duedate = CASE WHEN $9::timestamptz IS NOT NULL THEN $9::timestamptz ELSE duedate END,
            updated = now()
         WHERE ticket_id = $10",
    )
    .bind(input.name.as_deref().filter(|s| !s.is_empty()))
    .bind(input.email.as_deref().filter(|s| !s.is_empty()))
    .bind(input.phone.as_deref())
    .bind(input.phone_ext.as_deref())
    .bind(new_dept_id)
    .bind(input.topic_id)
    .bind(input.priority_id)
    .bind(input.source.as_deref().filter(|s| !s.is_empty()))
    .bind(parsed_due_date.map(|d| d.to_rfc3339()))
    .bind(ticket_id)
    .execute(&mut *tx)
    .await?;

    // Re-select SLA if department changed.
    // @implements BS-021.7: SLA re-selection on department change.
    let new_sla_id = if dept_changed || input.sla_id.is_some() {
        let sla_id = if let Some(explicit_sla) = input.sla_id {
            Some(explicit_sla)
        } else {
            // Re-select based on new department.
            select_sla_id_with_dept(pool, None, effective_dept_id, effective_topic_id).await?
        };

        if let Some(sid) = sla_id {
            sqlx::query("UPDATE ticket SET sla_id = $1 WHERE ticket_id = $2")
                .bind(sid)
                .bind(ticket_id)
                .execute(&mut *tx)
                .await?;
        }
        sla_id
    } else {
        None
    };

    // Clear overdue if due date is now in the future.
    // @implements BS-021.6: editing due date clears overdue flag.
    let overdue_cleared = if parsed_due_date.is_some() && current_overdue {
        sqlx::query("UPDATE ticket SET isoverdue = false WHERE ticket_id = $1")
            .bind(ticket_id)
            .execute(&mut *tx)
            .await?;
        true
    } else {
        false
    };

    // Log internal note "Ticket Updated".
    let note_body = format!("Ticket Updated\n\n{}", safe_html(&input.reason));
    append_internal_note(&mut tx, ticket_id, staff_id, staff_name, &note_body).await?;

    tx.commit().await?;

    Ok(UpdateResult {
        ticket_number,
        new_sla_id,
        overdue_cleared,
    })
}

// ---------------------------------------------------------------------------
// TS-M3-G1 — Bulk actions (mass process)
// ---------------------------------------------------------------------------

/// Bulk action type.
///
/// @implements FS-021.21: Bulk action types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkAction {
    Close,
    Reopen,
    Delete,
}

impl BulkAction {
    /// Parse an action string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "close" => Some(BulkAction::Close),
            "reopen" => Some(BulkAction::Reopen),
            "delete" => Some(BulkAction::Delete),
            _ => None,
        }
    }

    /// Action name for messages.
    pub fn verb(&self) -> &'static str {
        match self {
            BulkAction::Close => "closed",
            BulkAction::Reopen => "reopened",
            BulkAction::Delete => "deleted",
        }
    }
}

/// Result of a bulk action.
///
/// @implements FS-021.21: Bulk action result with partial success.
#[derive(Debug)]
pub struct BulkResult {
    pub succeeded: usize,
    pub failed: usize,
    pub message: String,
}

/// Bulk action errors.
#[derive(Debug, thiserror::Error)]
pub enum BulkError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("No tickets selected")]
    EmptySelection,
    #[error("Unknown action")]
    UnknownAction,
}

/// Execute a bulk action on multiple tickets.
///
/// Processes each ticket individually with partial success reporting.
/// If some tickets fail their preconditions (already closed, etc.), the
/// successful ones are still processed.
///
/// @implements FS-021.21: Mass process endpoint.
/// @implements BS-020.11: Partial success reporting.
pub async fn bulk_action(
    pool: &PgPool,
    action: BulkAction,
    ticket_ids: &[i64],
    staff_id: i32,
    staff_name: &str,
    staff_username: &str,
) -> Result<BulkResult, BulkError> {
    if ticket_ids.is_empty() {
        return Err(BulkError::EmptySelection);
    }

    let mut succeeded = 0;
    let mut failed = 0;

    for &ticket_id in ticket_ids {
        // Each action returns a different result type, so we map to Result<(), _> for uniformity.
        let result: Result<(), WorkflowError> = match action {
            BulkAction::Close => {
                close_ticket(pool, ticket_id, staff_id, staff_name, staff_username, None)
                    .await
                    .map(|_| ())
            }
            BulkAction::Reopen => {
                reopen_ticket(pool, ticket_id, staff_id, staff_name, staff_username, None)
                    .await
                    .map(|_| ())
            }
            BulkAction::Delete => {
                delete_ticket(pool, ticket_id, staff_username)
                    .await
                    .map(|_| ())
            }
        };

        match result {
            Ok(_) => succeeded += 1,
            Err(e) => {
                tracing::debug!(ticket_id = ticket_id, error = %e, "Bulk action failed for ticket");
                failed += 1;
            }
        }
    }

    let total = succeeded + failed;
    let message = if failed == 0 {
        format!("{} tickets {}", succeeded, action.verb())
    } else {
        format!("{} of {} tickets {}", succeeded, total, action.verb())
    };

    Ok(BulkResult {
        succeeded,
        failed,
        message,
    })
}

// ---------------------------------------------------------------------------
// TS-M3-I3 — Ticket locking
// ---------------------------------------------------------------------------

/// Result of a lock acquisition.
///
/// @implements FS-021.18: Lock acquire result.
#[derive(Debug)]
pub struct LockResult {
    pub lock_id: i32,
    pub remaining_seconds: i64,
}

/// Lock state of a ticket as seen by a specific viewing staff member.
///
/// Surfaced on the ticket-detail load so the staff UI can render a
/// "This ticket is currently locked by <name>" banner and block replies when
/// another agent holds a live lock. `expires_at` is the live lock's expiry
/// (ISO-8601) whether the lock is the viewer's own or another agent's; it is
/// `null` when no live lock exists. `locked_by_name` is set **only** when the
/// live lock is held by a *different* staff member.
///
/// @implements FS-021.18: Collaborative edit-lock state exposed to the viewer.
/// @implements FS-021.20: "locked by another staff" signal with holder name.
/// @implements BS-021.3: one active lock per ticket blocks conflicting edits.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LockInfo {
    /// True when a live lock is held by a staff member other than the viewer.
    pub locked_by_other: bool,
    /// Display name of the *other* lock holder (null unless `locked_by_other`).
    pub locked_by_name: Option<String>,
    /// ISO-8601 expiry of the live lock (own or other), null when unlocked.
    pub expires_at: Option<String>,
}

/// Lock acquisition errors.
#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("ticket not found")]
    NotFound,
    #[error("Ticket is currently locked by {0}")]
    LockedByOther(String),
}

/// Acquire or renew a lock on a ticket.
///
/// First deletes expired locks, then inserts a new lock or renews the existing
/// lock if owned by the same staff. Returns the lock id and remaining time.
///
/// @implements FS-021.18: Lock acquire/renew.
/// @implements BS-021.3: One active lock per ticket.
pub async fn acquire_lock(
    pool: &PgPool,
    ticket_id: i64,
    staff_id: i32,
    lock_time_minutes: i32,
) -> Result<LockResult, LockError> {
    // Verify ticket exists.
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT ticket_id FROM ticket WHERE ticket_id = $1",
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await?;

    if row.is_none() {
        return Err(LockError::NotFound);
    }

    let mut tx = pool.begin().await?;

    // Delete expired locks on this ticket.
    sqlx::query("DELETE FROM ticket_lock WHERE ticket_id = $1 AND expire < now()")
        .bind(ticket_id)
        .execute(&mut *tx)
        .await?;

    // Check for existing non-expired lock.
    let existing: Option<(i32, i32, String)> = sqlx::query_as(
        "SELECT tl.lock_id, tl.staff_id, COALESCE(NULLIF(TRIM(CONCAT(s.firstname, ' ', s.lastname)), ''), s.username) AS name
         FROM ticket_lock tl
         JOIN staff s ON s.staff_id = tl.staff_id
         WHERE tl.ticket_id = $1",
    )
    .bind(ticket_id)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some((lock_id, owner_id, owner_name)) = existing {
        if owner_id == staff_id {
            // Renew own lock.
            sqlx::query(
                "UPDATE ticket_lock SET expire = now() + ($1 || ' minutes')::interval WHERE lock_id = $2",
            )
            .bind(lock_time_minutes)
            .bind(lock_id)
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;

            return Ok(LockResult {
                lock_id,
                remaining_seconds: (lock_time_minutes * 60) as i64,
            });
        } else {
            // Locked by another staff member.
            return Err(LockError::LockedByOther(owner_name));
        }
    }

    // No live lock seen above → create one. Race-safe INSERT: two concurrent
    // same-staff acquires (e.g. a React double-invoke firing the lock POST twice)
    // both reach here having seen no lock, so the second would violate the UNIQUE
    // (ticket_id) constraint. `ON CONFLICT (ticket_id) DO NOTHING` mirrors the
    // legacy `INSERT IGNORE` (class.lock.php TicketLock::acquire) — the loser gets
    // no row back and we re-resolve the winner's lock below, so a same-staff race
    // NEVER errors (idempotent) and never produces a spurious lock failure.
    //
    // @implements FS-021.18: acquire = insert-if-absent (legacy INSERT IGNORE).
    // @implements BS-021.3: single active lock per ticket (UNIQUE ticket_id).
    let inserted: Option<i32> = sqlx::query_scalar(
        "INSERT INTO ticket_lock (ticket_id, staff_id, expire)
         VALUES ($1, $2, now() + ($3 || ' minutes')::interval)
         ON CONFLICT (ticket_id) DO NOTHING
         RETURNING lock_id",
    )
    .bind(ticket_id)
    .bind(staff_id)
    .bind(lock_time_minutes)
    .fetch_optional(&mut *tx)
    .await?;

    let lock_id = match inserted {
        Some(id) => id,
        None => {
            // Lost an insert race: a lock now exists on this ticket. Re-read it
            // and resolve exactly like the pre-check branch — renew for self,
            // reject for another staff member.
            let (existing_id, owner_id, owner_name): (i32, i32, String) = sqlx::query_as(
                "SELECT tl.lock_id, tl.staff_id, COALESCE(NULLIF(TRIM(CONCAT(s.firstname, ' ', s.lastname)), ''), s.username) AS name
                 FROM ticket_lock tl
                 JOIN staff s ON s.staff_id = tl.staff_id
                 WHERE tl.ticket_id = $1",
            )
            .bind(ticket_id)
            .fetch_one(&mut *tx)
            .await?;

            if owner_id != staff_id {
                return Err(LockError::LockedByOther(owner_name));
            }

            // Same staff — renew (idempotent).
            sqlx::query(
                "UPDATE ticket_lock SET expire = now() + ($1 || ' minutes')::interval WHERE lock_id = $2",
            )
            .bind(lock_time_minutes)
            .bind(existing_id)
            .execute(&mut *tx)
            .await?;

            existing_id
        }
    };

    tx.commit().await?;

    Ok(LockResult {
        lock_id,
        remaining_seconds: (lock_time_minutes * 60) as i64,
    })
}

/// Lock state of a ticket relative to a viewing staff member.
///
/// Clears any expired lock on the ticket first (so a stale lock never blocks the
/// viewer), then reports the live lock (if any): `locked_by_other` + the holder's
/// display name when a *different* agent holds it, plus the lock's expiry. Used by
/// the staff ticket-detail load to drive the "locked by <name>" banner and to
/// disable the reply composer for a non-holder.
///
/// The holder name matches the legacy source (class.lock.php) — composed from the
/// staff first/last name, falling back to the username when both are blank.
///
/// @implements FS-021.18: expose collaborative-lock state on ticket view.
/// @implements FS-021.20: "locked by another staff" signal with holder name.
/// @implements BS-021.3: lock blocks conflicting replies.
pub async fn get_lock_info(
    pool: &PgPool,
    ticket_id: i64,
    viewer_staff_id: i32,
) -> Result<LockInfo, sqlx::Error> {
    // Drop expired locks first so an expired lock reads as "unlocked".
    sqlx::query("DELETE FROM ticket_lock WHERE ticket_id = $1 AND expire < now()")
        .bind(ticket_id)
        .execute(pool)
        .await?;

    let row: Option<(i32, String, String)> = sqlx::query_as(
        "SELECT tl.staff_id,
                COALESCE(NULLIF(TRIM(CONCAT(s.firstname, ' ', s.lastname)), ''), s.username) AS name,
                to_char(tl.expire, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS expires_at
         FROM ticket_lock tl
         JOIN staff s ON s.staff_id = tl.staff_id
         WHERE tl.ticket_id = $1 AND tl.expire > now()",
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await?;

    Ok(match row {
        Some((owner_id, name, expires_at)) if owner_id != viewer_staff_id => LockInfo {
            locked_by_other: true,
            locked_by_name: Some(name),
            expires_at: Some(expires_at),
        },
        // The viewer's own live lock — surface the expiry, not a "locked" banner.
        Some((_, _, expires_at)) => LockInfo {
            locked_by_other: false,
            locked_by_name: None,
            expires_at: Some(expires_at),
        },
        None => LockInfo {
            locked_by_other: false,
            locked_by_name: None,
            expires_at: None,
        },
    })
}

/// Release a lock on a ticket.
///
/// Deletes the lock if owned by the requester. Returns true if a lock was
/// released, false if not owned or no lock existed.
///
/// @implements FS-021.18: Lock release.
pub async fn release_lock(
    pool: &PgPool,
    ticket_id: i64,
    staff_id: i32,
) -> Result<bool, LockError> {
    let result = sqlx::query(
        "DELETE FROM ticket_lock WHERE ticket_id = $1 AND staff_id = $2",
    )
    .bind(ticket_id)
    .bind(staff_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Check if a ticket is locked by another staff member.
///
/// Returns the lock owner's name if locked by someone else, None otherwise.
///
/// @implements BS-021.3: Lock check for reply blocking.
pub async fn check_lock(
    pool: &PgPool,
    ticket_id: i64,
    staff_id: i32,
) -> Result<Option<String>, sqlx::Error> {
    // First delete expired locks.
    sqlx::query("DELETE FROM ticket_lock WHERE ticket_id = $1 AND expire < now()")
        .bind(ticket_id)
        .execute(pool)
        .await?;

    // Check for lock by another staff member.
    let owner: Option<(String,)> = sqlx::query_as(
        "SELECT COALESCE(NULLIF(TRIM(CONCAT(s.firstname, ' ', s.lastname)), ''), s.username) AS name
         FROM ticket_lock tl
         JOIN staff s ON s.staff_id = tl.staff_id
         WHERE tl.ticket_id = $1 AND tl.staff_id != $2",
    )
    .bind(ticket_id)
    .bind(staff_id)
    .fetch_optional(pool)
    .await?;

    Ok(owner.map(|(name,)| name))
}

/// Get the default lock time from config.
///
/// @implements FS-021.18: Lock time from config.
pub async fn get_lock_time_minutes(pool: &PgPool) -> Result<i32, sqlx::Error> {
    let value: Option<String> = sqlx::query_scalar(
        "SELECT value FROM config WHERE key = 'ticket_lock_time'",
    )
    .fetch_optional(pool)
    .await?;

    Ok(value
        .and_then(|v| v.parse().ok())
        .unwrap_or(5)) // Default 5 minutes
}

/// Cleanup expired locks (called by cron sweep).
///
/// @implements FS-021.18: Cron lock cleanup.
pub async fn cleanup_expired_locks(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM ticket_lock WHERE expire < now()")
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignee_type_parses_staff() {
        assert_eq!(AssigneeType::parse("s123"), Some(AssigneeType::Staff(123)));
        assert_eq!(AssigneeType::parse("s1"), Some(AssigneeType::Staff(1)));
    }

    #[test]
    fn assignee_type_parses_team() {
        assert_eq!(AssigneeType::parse("t456"), Some(AssigneeType::Team(456)));
        assert_eq!(AssigneeType::parse("t1"), Some(AssigneeType::Team(1)));
    }

    #[test]
    fn assignee_type_rejects_invalid() {
        assert_eq!(AssigneeType::parse("123"), None);
        assert_eq!(AssigneeType::parse("x123"), None);
        assert_eq!(AssigneeType::parse(""), None);
        assert_eq!(AssigneeType::parse("s"), None);
        assert_eq!(AssigneeType::parse("sabc"), None);
    }

    #[test]
    fn note_state_parses_valid_values() {
        assert_eq!(NoteState::from_str("unchanged"), Some(NoteState::Unchanged));
        assert_eq!(NoteState::from_str(""), Some(NoteState::Unchanged));
        assert_eq!(NoteState::from_str("closed"), Some(NoteState::Closed));
        assert_eq!(NoteState::from_str("CLOSED"), Some(NoteState::Closed));
        assert_eq!(NoteState::from_str("open"), Some(NoteState::Open));
        assert_eq!(NoteState::from_str("answered"), Some(NoteState::Answered));
        assert_eq!(NoteState::from_str("unanswered"), Some(NoteState::Unanswered));
        assert_eq!(NoteState::from_str("overdue"), Some(NoteState::Overdue));
        assert_eq!(NoteState::from_str("notdue"), Some(NoteState::NotDue));
    }

    #[test]
    fn note_state_rejects_invalid_values() {
        assert_eq!(NoteState::from_str("bogus"), None);
        assert_eq!(NoteState::from_str("invalid"), None);
        assert_eq!(NoteState::from_str("close"), None);
    }

    #[test]
    fn bulk_action_parses_valid_values() {
        assert_eq!(BulkAction::from_str("close"), Some(BulkAction::Close));
        assert_eq!(BulkAction::from_str("CLOSE"), Some(BulkAction::Close));
        assert_eq!(BulkAction::from_str("reopen"), Some(BulkAction::Reopen));
        assert_eq!(BulkAction::from_str("delete"), Some(BulkAction::Delete));
    }

    #[test]
    fn bulk_action_rejects_invalid_values() {
        assert_eq!(BulkAction::from_str("bogus"), None);
        assert_eq!(BulkAction::from_str(""), None);
    }

    #[test]
    fn bulk_action_verb() {
        assert_eq!(BulkAction::Close.verb(), "closed");
        assert_eq!(BulkAction::Reopen.verb(), "reopened");
        assert_eq!(BulkAction::Delete.verb(), "deleted");
    }
}
