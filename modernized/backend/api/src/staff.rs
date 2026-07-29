//! Staff-realm read/profile routes (TS-M1-C1/C2/C3, TS-M3-A1, TS-M3-A2, TS-M3-B2).
//!
//! All routes here are gated by the staff realm (the [`StaffSession`] extractor,
//! TS-M1-A4b); mutating routes additionally enforce CSRF + a named permission.
//!
//! @implements BS-002: authenticated staff profile (`GET /api/staff/me`).
//! @implements BS-020.1: status param handling (queue selection).
//! @implements BS-020.2: visibility scoping (dept + assignment predicate).
//! @implements BS-020.8: sticky per-queue sort preferences (session-based).
//! @implements FS-020.11: quick-stats endpoint.

use axum::extract::{FromRequest, Multipart, Path, Query, Request, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::fmt;

use ost_core::session::{QueueSort, SessionData, SortPreferences};

use ost_core::attachment::load_attachments_by_ref;
use ost_core::canned::{load_offerable_response, load_ticket_vars};
use ost_core::permission::PERM_CAN_POST_REPLY;
use ost_core::ticket::{load_thread, post_staff_reply, NewThreadEntry};
use ost_core::variable::VariableReplacer;
use ost_core::visibility::{load_staff_visibility, StaffVisibility};
use ost_core::{ApiError, AttachmentSpec, AttachmentView};

use crate::attachments::{drain_multipart, load_upload_policy, validate_attachment};
use crate::auth::gate::require_staff_permission;
use crate::auth::realm::StaffSession;
use crate::auth::StaffCsrf;
use crate::canned::{CFG_HELPDESK_URL, DEFAULT_HELPDESK_URL};
use crate::config_keys::read_config;
use crate::state::AppState;

/// A ticket header row as selected by the detail + reply routes:
/// `(ticket_id, number, subject, email, name, status, created, isanswered, isoverdue, duedate, sla_id, sla_name, staff_id)`.
///
/// @implements BS-032.10: includes duedate, sla_id, sla_name.
/// @implements TS-M3-C4: includes staff_id for assignment check.
type TicketHeader = (
    i64,            // ticket_id
    i64,            // number (ticketID)
    String,         // subject
    String,         // email
    String,         // name
    String,         // status
    String,         // created
    bool,           // isanswered
    bool,           // isoverdue
    Option<String>, // duedate
    Option<i32>,    // sla_id
    Option<String>, // sla_name
    Option<i32>,    // staff_id
);

/// The authenticated staff profile returned by `GET /api/staff/me`.
///
/// @implements FS-032.1: exposes `isadmin` + the four M4 capability flags so the
///   TS-M4-A0 frontend guards can gate the admin panel. The four capability flags
///   use snake_case names (the pinned frontend contract) via explicit renames.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffMe {
    pub id: i32,
    pub username: String,
    /// Display name (`firstname lastname`, trimmed; falls back to username).
    pub name: String,
    pub dept_id: i32,
    // Permission flags from the staff member's group.
    pub can_create_tickets: bool,
    pub can_close_tickets: bool,
    pub can_delete_tickets: bool,
    pub can_edit_tickets: bool,
    pub can_assign_tickets: bool,
    pub can_transfer_tickets: bool,
    pub can_manage_tickets: bool,
    /// Whether this staff account is an administrator (`staff.isadmin`).
    /// @implements FS-032.1: admin flag on the profile.
    pub isadmin: bool,
    /// M4 capability flags (snake_case per the pinned TS-M4-A0 contract).
    #[serde(rename = "can_manage_faq")]
    pub can_manage_faq: bool,
    #[serde(rename = "can_manage_premade")]
    pub can_manage_premade: bool,
    #[serde(rename = "can_ban_emails")]
    pub can_ban_emails: bool,
    #[serde(rename = "can_view_staff_stats")]
    pub can_view_staff_stats: bool,
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

    // Load permission flags from the staff member's group.
    let store = state
        .sessions
        .as_ref()
        .ok_or_else(|| ApiError::internal("Session store is unavailable"))?;
    let (isadmin, perms) = store
        .staff_capabilities(session.staff_id)
        .await
        .map_err(|_| ApiError::internal("Permission lookup failed"))?
        .ok_or_else(|| ApiError::unauthenticated("Session expired or invalid"))?;

    Ok(Json(StaffMe {
        id: session.staff_id,
        username,
        name,
        dept_id,
        can_create_tickets: perms.group_enabled && perms.can_create_tickets,
        can_close_tickets: perms.group_enabled && perms.can_close_tickets,
        can_delete_tickets: perms.group_enabled && perms.can_delete_tickets,
        can_edit_tickets: perms.group_enabled && perms.can_edit_tickets,
        can_assign_tickets: perms.group_enabled && perms.can_assign_tickets,
        can_transfer_tickets: perms.group_enabled && perms.can_transfer_tickets,
        can_manage_tickets: perms.group_enabled && perms.can_manage_tickets,
        // @implements FS-032.1: admin flag + M4 capability flags on the profile.
        // isadmin comes straight from the account; the four capability flags are
        // group-gated (a disabled group grants nothing).
        isadmin,
        can_manage_faq: perms.group_enabled && perms.can_manage_faq,
        can_manage_premade: perms.group_enabled && perms.can_manage_premade,
        can_ban_emails: perms.group_enabled && perms.can_ban_emails,
        can_view_staff_stats: perms.group_enabled && perms.can_view_staff_stats,
    }))
}

/// `GET /api/staff/ticket-options` — the reference lists an agent needs to power
/// the ticket Transfer / Assign / Edit-properties dialogs, under a plain staff
/// gate (any authenticated staff, NOT admin). Read-only, exposes only id + name
/// (no secrets), so it is safe outside the `/admin/*` surface; the admin
/// form-options endpoints are re-used for their query logic but require admin.
///
/// Response shape (each value is `[{ "id", "name" }]`):
/// - `departments`  — all departments (Transfer target list).
/// - `agents`       — assignable staff: active staff, `name` = display name.
/// - `teams`        — assignable teams: enabled teams.
/// - `help_topics`  — active help topics (Edit properties).
/// - `priorities`   — the priority set, ordered by urgency DESC.
/// - `sla_plans`    — active SLA plans (Edit properties).
///
/// @implements FS-032.10/.11/.12: staff transfer/assign/edit reference data.
/// @implements BS-020.2: read-only reference lists available to any staff realm
///   session (not gated behind the admin capability).
pub async fn ticket_options(
    State(state): State<AppState>,
    _session: StaffSession,
) -> Result<Json<serde_json::Value>, ApiError> {
    use serde_json::json;

    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Ticket options are unavailable"))?;

    type NameRow = (i32, String);
    let id_name = |rows: Vec<NameRow>| -> Vec<serde_json::Value> {
        rows.into_iter()
            .map(|(id, name)| json!({ "id": id, "name": name }))
            .collect()
    };

    // departments: all departments (Transfer target list).
    let dept_rows: Vec<NameRow> =
        sqlx::query_as("SELECT dept_id, dept_name FROM department ORDER BY dept_name ASC")
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::internal("Department lookup failed"))?;

    // agents: active staff; name = display name (firstname+lastname, else username).
    let agent_rows: Vec<NameRow> = sqlx::query_as(
        "SELECT staff_id, COALESCE(NULLIF(TRIM(firstname || ' ' || lastname), ''), username) AS name \
         FROM staff WHERE isactive = true ORDER BY name ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Staff lookup failed"))?;

    // teams: enabled teams.
    let team_rows: Vec<NameRow> =
        sqlx::query_as("SELECT team_id, name FROM team WHERE isenabled = true ORDER BY name ASC")
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::internal("Team lookup failed"))?;

    // help_topics: active topics.
    let topic_rows: Vec<NameRow> = sqlx::query_as(
        "SELECT topic_id, topic FROM help_topic WHERE isactive = true ORDER BY topic ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Help topic lookup failed"))?;

    // priorities: full set, most-urgent first.
    let priority_rows: Vec<NameRow> = sqlx::query_as(
        "SELECT priority_id, priority_desc FROM priority ORDER BY urgency DESC, priority_id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Priority lookup failed"))?;

    // sla_plans: active SLA plans.
    let sla_rows: Vec<NameRow> =
        sqlx::query_as("SELECT id, name FROM sla WHERE isactive = true ORDER BY name ASC")
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::internal("SLA lookup failed"))?;

    Ok(Json(json!({
        "departments": id_name(dept_rows),
        "agents": id_name(agent_rows),
        "teams": id_name(team_rows),
        "help_topics": id_name(topic_rows),
        "priorities": id_name(priority_rows),
        "sla_plans": id_name(sla_rows),
    })))
}

/// One row of the staff open-tickets queue.
///
/// @implements BS-020.5: includes dynamic rightmost column fields.
/// @implements BS-032.10: includes isoverdue and duedate per row.
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
    /// Whether the ticket has been answered (any staff reply -> true, FS-020.3). Drives
    /// the queue's Answered / Unanswered badge.
    pub isanswered: bool,
    /// Whether the ticket is overdue.
    ///
    /// @implements BS-032.10: isoverdue flag in listing response.
    pub isoverdue: bool,
    /// Due date (ISO 8601 timestamp), null if no SLA or no explicit due date.
    ///
    /// @implements BS-032.10: duedate field in listing response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duedate: Option<String>,
    /// Staff or team name assigned to this ticket. Shown when rightmost_column=assigned_to.
    /// Staff name takes precedence over team name (BS-020.5).
    ///
    /// @implements BS-020.5: Assigned To column (staff name or team name).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_to_name: Option<String>,
    /// Name of the staff who closed this ticket. Shown when rightmost_column=closed_by.
    ///
    /// @implements BS-020.5: Closed By column.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_by_name: Option<String>,
    /// Department name. Shown when rightmost_column=department.
    ///
    /// @implements BS-020.5: Department column.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub department_name: Option<String>,
}

/// The rightmost column type for the queue response.
///
/// @implements BS-020.5: rightmost column varies by queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightmostColumn {
    /// Show "Assigned To" (staff or team name).
    AssignedTo,
    /// Show "Closed By" (closing staff name).
    ClosedBy,
    /// Show "Department" (department name).
    Department,
}

impl RightmostColumn {
    /// Convert to the wire format string.
    pub fn as_str(&self) -> &'static str {
        match self {
            RightmostColumn::AssignedTo => "assigned_to",
            RightmostColumn::ClosedBy => "closed_by",
            RightmostColumn::Department => "department",
        }
    }
}

/// Queue status selector for the `status` query parameter (BS-020.1).
///
/// The `status` param is "overloaded": it selects a queue, not a raw ticket status.
/// - `open` (default) -> `status='open'` + answered/assigned visibility rules
/// - `answered` -> `status='open' AND isanswered=true`
/// - `assigned` -> `status='open' AND staff_id=me` (My Tickets)
/// - `overdue` -> `status='open' AND isoverdue=true`
/// - `closed` -> `status='closed'`
///
/// @implements BS-020.1: status param handling (queue selection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueueStatus {
    #[default]
    Open,
    Answered,
    Assigned,
    Overdue,
    Closed,
}

impl fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueueStatus::Open => write!(f, "open"),
            QueueStatus::Answered => write!(f, "answered"),
            QueueStatus::Assigned => write!(f, "assigned"),
            QueueStatus::Overdue => write!(f, "overdue"),
            QueueStatus::Closed => write!(f, "closed"),
        }
    }
}

/// Query parameters for the ticket queue endpoint.
///
/// @implements BS-020.1: status param handling.
/// @implements AC-8: Unknown status value defaults to open.
/// @implements FS-020.5: sort and order query params.
/// @implements FS-020.7: basic search (query param).
/// @implements FS-020.8: advanced search (status, deptId, assignee, staffId, topicId, startDate, endDate).
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueParams {
    /// Action: "search" triggers search mode.
    /// @implements FS-020.7: a=search triggers search.
    #[serde(default)]
    pub a: Option<String>,
    /// Queue selector: open (default), answered, assigned, overdue, closed.
    /// Unknown values default to `open` (AC-8).
    #[serde(default)]
    pub status: Option<String>,
    /// Sort key: date, ID, pri, name, subj, status, assignee, staff, dept.
    /// Invalid/unknown values fall through to the queue's default sort.
    #[serde(default)]
    pub sort: Option<String>,
    /// Sort direction: ASC or DESC. Invalid values default to DESC.
    #[serde(default)]
    pub order: Option<String>,
    /// Page number (1-based). Invalid/missing defaults to 1.
    #[serde(default)]
    pub p: Option<String>,
    /// Page size override. Clamped to 5..=100.
    #[serde(default)]
    pub limit: Option<String>,
    // --- Search parameters (FS-020.7 / FS-020.8) ---
    /// Search keyword (>= 3 chars). Numeric -> ticket number prefix, email -> exact match,
    /// otherwise deep text search across email/name/subject/thread body.
    /// @implements BS-020.7: keyword resolution strategy.
    #[serde(default)]
    pub query: Option<String>,
    /// Department ID filter (advanced search). Out-of-scope deptId is silently ignored.
    /// @implements BS-020.3: deptId bounded by staff access.
    #[serde(default, rename = "deptId")]
    pub dept_id: Option<i32>,
    /// Assignee filter: "s{id}" for staff, "t{id}" for team, "s0" for unassigned.
    /// Ignored when status=closed.
    /// @implements FS-020.8: assignee filter.
    #[serde(default)]
    pub assignee: Option<String>,
    /// Closed-by staff ID filter (advanced search).
    /// @implements FS-020.8: staffId (closed-by) filter.
    #[serde(default, rename = "staffId")]
    pub staff_id: Option<i32>,
    /// Help topic ID filter (advanced search).
    /// @implements FS-020.8: topicId filter.
    #[serde(default, rename = "topicId")]
    pub topic_id: Option<i32>,
    /// Start date bound for created date (inclusive).
    /// @implements FS-020.8: startDate filter.
    #[serde(default, rename = "startDate")]
    pub start_date: Option<String>,
    /// End date bound for created date (inclusive).
    /// @implements FS-020.8: endDate filter.
    #[serde(default, rename = "endDate")]
    pub end_date: Option<String>,
}

/// Valid sort keys and their corresponding SQL expressions.
///
/// @implements FS-020.5: sortable columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Date,
    Id,
    Priority,
    Name,
    Subject,
    Status,
    Assignee,
    Staff,
    Department,
}

impl SortKey {
    /// Parse a sort key string into a SortKey enum.
    /// Returns None for invalid/unknown keys.
    ///
    /// @implements FS-020.5: sort key validation.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "date" => Some(SortKey::Date),
            "ID" => Some(SortKey::Id),
            "pri" => Some(SortKey::Priority),
            "name" => Some(SortKey::Name),
            "subj" => Some(SortKey::Subject),
            "status" => Some(SortKey::Status),
            "assignee" => Some(SortKey::Assignee),
            "staff" => Some(SortKey::Staff),
            "dept" => Some(SortKey::Department),
            _ => None,
        }
    }

    /// The SQL expression for this sort key.
    ///
    /// @implements FS-020.5: sort key to column mapping.
    pub fn sql_expression(&self) -> &'static str {
        match self {
            SortKey::Date => "ticket.created",
            SortKey::Id => r#"ticket."ticketID"::integer"#,
            SortKey::Priority => "COALESCE(priority.urgency, 2)",
            SortKey::Name => "ticket.name",
            SortKey::Subject => "ticket.subject",
            SortKey::Status => "ticket.status",
            SortKey::Assignee => "COALESCE(staff.firstname || ' ' || staff.lastname, '')",
            SortKey::Staff => "COALESCE(closing_staff.firstname || ' ' || closing_staff.lastname, '')",
            SortKey::Department => "COALESCE(department.dept_name, '')",
        }
    }

    /// Whether this sort key requires a JOIN on the priority table.
    pub fn requires_priority_join(&self) -> bool {
        matches!(self, SortKey::Priority)
    }

    /// Whether this sort key requires a JOIN on the staff table (assignee).
    pub fn requires_staff_join(&self) -> bool {
        matches!(self, SortKey::Assignee)
    }

    /// Whether this sort key requires a JOIN on the staff table (closing staff).
    pub fn requires_closing_staff_join(&self) -> bool {
        matches!(self, SortKey::Staff)
    }

    /// Whether this sort key requires a JOIN on the department table.
    pub fn requires_dept_join(&self) -> bool {
        matches!(self, SortKey::Department)
    }
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

impl SortOrder {
    /// Parse order string. Invalid values default to DESC.
    ///
    /// @implements FS-020.5: order direction validation (default DESC).
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "ASC" => SortOrder::Asc,
            _ => SortOrder::Desc,
        }
    }

    pub fn sql(&self) -> &'static str {
        match self {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        }
    }
}

/// Resolved sort configuration for a query.
#[derive(Debug, Clone)]
pub struct SortConfig {
    /// The explicit sort key (if valid), or None to use queue default.
    pub key: Option<SortKey>,
    /// The sort order (defaults to DESC).
    pub order: SortOrder,
}

impl QueueParams {
    /// Parse the status string into a QueueStatus enum, defaulting to Open for
    /// unknown/missing values (AC-8).
    ///
    /// @implements AC-8: Unknown status value defaults to open.
    pub fn queue_status(&self) -> QueueStatus {
        match self.status.as_deref() {
            Some("open") => QueueStatus::Open,
            Some("answered") => QueueStatus::Answered,
            Some("assigned") => QueueStatus::Assigned,
            Some("overdue") => QueueStatus::Overdue,
            Some("closed") => QueueStatus::Closed,
            _ => QueueStatus::Open, // Default for unknown or missing
        }
    }

    /// Parse sort parameters into a SortConfig.
    ///
    /// @implements FS-020.5: sort param parsing.
    pub fn sort_config(&self) -> SortConfig {
        let key = self.sort.as_deref().and_then(SortKey::from_str);
        let order = self
            .order
            .as_deref()
            .map(SortOrder::from_str)
            .unwrap_or_default();
        SortConfig { key, order }
    }

    /// Parse pagination parameters.
    ///
    /// @implements FS-020.6: page number parsing (default 1).
    pub fn page(&self) -> u32 {
        self.p
            .as_deref()
            .and_then(|s| s.parse::<i32>().ok())
            .filter(|&p| p > 0)
            .map(|p| p as u32)
            .unwrap_or(1)
    }

    /// Parse limit parameter, clamped to 5..=100.
    ///
    /// @implements FS-020.6: limit parsing with clamping.
    pub fn limit(&self) -> Option<u32> {
        self.limit
            .as_deref()
            .and_then(|s| s.parse::<i32>().ok())
            .map(|l| l.clamp(5, 100) as u32)
    }

    /// Check if search mode is active (a=search).
    ///
    /// @implements FS-020.7: search mode detection.
    pub fn is_search(&self) -> bool {
        self.a.as_deref() == Some("search")
    }

    /// Get the search status filter (for advanced search).
    /// Returns None if status is "any" or unset, otherwise the status value.
    ///
    /// @implements FS-020.8: status filter in advanced search.
    pub fn search_status(&self) -> Option<&str> {
        match self.status.as_deref() {
            Some("any") | None => None,
            Some(s) => Some(s),
        }
    }
}

/// Pagination metadata returned with queue results.
///
/// @implements FS-020.6: pagination metadata.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationInfo {
    /// Current page number (1-based).
    pub page: u32,
    /// Page size (items per page).
    pub page_size: u32,
    /// Total count of tickets matching the criteria (before pagination).
    pub total_count: i64,
    /// Total number of pages.
    pub total_pages: u32,
}

/// Response shape for the ticket queue endpoint.
///
/// @implements BS-020.1: Response shape `{ tickets, pagination, rightmost_column }`.
/// @implements FS-020.6: pagination metadata in response.
/// @implements FS-020.7: search_results flag in response.
/// @implements EC-020.10: status_column flag when status=any (no status filter).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueResponse {
    /// The list of tickets for the current page.
    pub tickets: Vec<QueueItem>,
    /// Pagination metadata.
    pub pagination: PaginationInfo,
    /// The rightmost column name for this queue (for dynamic column rendering).
    /// Currently always "created" for M3.
    pub rightmost_column: String,
    /// True when this is a search result (a=search was specified).
    /// @implements FS-020.7 AC-8: search_results flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_results: Option<bool>,
    /// True when status=any (no status filter), so frontend shows Status column instead of Priority.
    /// @implements EC-020.10: status_column flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_column: Option<bool>,
}

/// Quick-stats response for `GET /api/staff/tickets/stats`.
///
/// Returns counts per queue for the authenticated staff member.
///
/// @implements FS-020.11: quick-stats endpoint.
#[derive(Debug, Serialize)]
pub struct StaffStats {
    /// Count of open tickets (not answered, not overdue, not assigned) visible to the staff.
    pub open: i64,
    /// Count of open+answered tickets.
    pub answered: i64,
    /// Count of open+overdue tickets.
    pub overdue: i64,
    /// Count of tickets assigned to the staff member.
    pub assigned: i64,
    /// Count of closed tickets.
    pub closed: i64,
    /// When true, answered tickets are folded into the Open count; Answered tab is hidden.
    /// @implements BS-020.4: config visibility for frontend tab logic (TS-M3-A4).
    pub show_answered_tickets: bool,
    /// When true, assigned tickets are included in Open queue.
    /// @implements BS-020.4: config visibility for frontend tab logic (TS-M3-A4).
    pub show_assigned_tickets: bool,
}

/// `GET /api/staff/tickets` — list tickets filtered by queue status with sorting and visibility.
///
/// The `status` query parameter selects which queue to display (BS-020.1):
/// - `open` (default): open tickets, optionally filtered by visibility toggles
/// - `answered`: open tickets that have been answered
/// - `assigned`: open tickets assigned to the authenticated staff member
/// - `overdue`: open tickets that are overdue
/// - `closed`: closed tickets
///
/// When `a=search` is provided, performs search instead of queue listing (FS-020.7/FS-020.8).
///
/// The `sort` and `order` params control ordering (FS-020.5). When omitted,
/// the session's sticky sort is used if set, otherwise per-queue default sorts apply.
///
/// When an explicit `sort` param is supplied (and is valid), the preference is stored
/// in the session for this queue (BS-020.8).
///
/// Unknown status values default to `open` (AC-8).
///
/// @implements BS-020.1: status param handling (queue selection).
/// @implements BS-020.2: visibility scoping (dept + assignment predicate).
/// @implements FS-020.5: sort/order param handling + per-queue default sorts.
/// @implements BS-020.8: sticky per-queue sort preferences (session-based).
/// @implements BS-020: open-tickets queue (number + subject + email + created).
/// @implements FS-020.7: basic search (a=search + query).
/// @implements FS-020.8: advanced search (multi-criteria filtering).
pub async fn list_tickets(
    State(state): State<AppState>,
    session: StaffSession,
    Query(params): Query<QueueParams>,
) -> Result<Json<QueueResponse>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Ticket queue is unavailable"))?;

    // Check for search mode (a=search).
    // @implements FS-020.7: a=search triggers search mode.
    if params.is_search() {
        return search_tickets(pool, session.staff_id, &params).await;
    }

    let queue_status = params.queue_status();
    let explicit_sort = params.sort_config();

    // BS-020.8: Resolve effective sort — explicit param > session pref > queue default.
    // If an explicit valid sort key was provided, use it and store in session.
    // Otherwise, check session for a sticky pref. Otherwise, fall back to default.
    let (effective_sort, should_store) = resolve_effective_sort(
        &explicit_sort,
        queue_status,
        session.sort_prefs.as_ref(),
        &params,
    );

    // BS-020.8: Store the explicit sort preference in session if valid.
    if should_store {
        if let Some(store) = state.sessions.as_ref() {
            store_sort_preference(
                store,
                &session.session_id,
                session.staff_id,
                session.sort_prefs.clone(),
                queue_status,
                &explicit_sort,
            )
            .await;
        }
    }

    // Load the staff member's visibility context (BS-020.2).
    // @implements TS-M3-A2: fetch dept_ids, team_ids, and show_assigned_only flag.
    let visibility = load_staff_visibility(pool, session.staff_id)
        .await
        .map_err(|_| ApiError::internal("Visibility lookup failed"))?;

    // Read visibility config for the open queue (BS-020.4)
    let show_assigned = read_config_bool(pool, "show_assigned_tickets").await;
    let show_answered = read_config_bool(pool, "show_answered_tickets").await;

    // Resolve the ORDER BY clause based on effective sort or queue defaults.
    // @implements FS-020.5: per-queue default sorts when no sort param is supplied.
    let order_by = resolve_order_by(queue_status, &effective_sort);

    // FS-020.6: Resolve pagination parameters.
    // Page size: limit param (clamped) > staff personal > system default > 25.
    let page = params.page();
    let page_size = resolve_page_size(pool, session.staff_id, params.limit()).await;

    // Determine the rightmost column based on queue type and config (BS-020.5).
    // @implements BS-020.5: rightmost column varies by queue.
    let rightmost_column = match queue_status {
        QueueStatus::Closed => RightmostColumn::ClosedBy,
        QueueStatus::Assigned => RightmostColumn::Department, // Assigned To is always "me"
        QueueStatus::Open => {
            if show_assigned {
                RightmostColumn::AssignedTo
            } else {
                RightmostColumn::Department
            }
        }
        QueueStatus::Answered | QueueStatus::Overdue => RightmostColumn::AssignedTo,
    };

    // Build the query based on queue status, applying visibility predicate and pagination.
    // @implements BS-020.2: visibility is always enforced on every queue.
    // @implements FS-020.6: pagination with LIMIT/OFFSET.
    // @implements BS-020.5: pass rightmost_column to queries for appropriate JOINs.
    let (items, total) = match queue_status {
        QueueStatus::Open => {
            query_open_queue_with_visibility(pool, show_assigned, show_answered, &visibility, &order_by, page, page_size, rightmost_column).await?
        }
        QueueStatus::Answered => query_answered_queue_with_visibility(pool, &visibility, &order_by, page, page_size, rightmost_column).await?,
        QueueStatus::Assigned => query_assigned_queue(pool, session.staff_id, &order_by, page, page_size).await?,
        QueueStatus::Overdue => query_overdue_queue_with_visibility(pool, &visibility, &order_by, page, page_size, rightmost_column).await?,
        QueueStatus::Closed => query_closed_queue_with_visibility(pool, &visibility, &order_by, page, page_size).await?,
    };

    // Calculate total pages.
    let total_pages = if total == 0 {
        1
    } else {
        ((total as u32 + page_size - 1) / page_size).max(1)
    };

    Ok(Json(QueueResponse {
        tickets: items,
        pagination: PaginationInfo {
            page,
            page_size,
            total_count: total,
            total_pages,
        },
        rightmost_column: rightmost_column.as_str().to_string(),
        search_results: None,
        status_column: None,
    }))
}

// ---------------------------------------------------------------------------
// Search functionality (TS-M3-F1, TS-M3-F2)
// ---------------------------------------------------------------------------

/// Keyword type determined from the search query.
///
/// @implements BS-020.7: keyword resolution strategy.
#[derive(Debug, Clone)]
enum KeywordType {
    /// Purely numeric query -> ticket number prefix match.
    TicketNumber(String),
    /// Contains @ and looks like email -> exact email match.
    Email(String),
    /// Otherwise -> deep text search across email/name/subject/thread body.
    DeepText(String),
}

impl KeywordType {
    /// Determine the keyword type from a search query.
    ///
    /// @implements BS-020.7: numeric -> ticket number prefix, email -> exact match,
    /// otherwise deep text search.
    fn from_query(query: &str) -> Self {
        let trimmed = query.trim();

        // Purely numeric -> ticket number prefix match.
        if trimmed.chars().all(|c| c.is_ascii_digit()) {
            return KeywordType::TicketNumber(trimmed.to_string());
        }

        // Contains @ and looks like a valid email -> exact email match.
        if trimmed.contains('@') && is_valid_email(trimmed) {
            return KeywordType::Email(trimmed.to_string());
        }

        // Otherwise -> deep text search.
        KeywordType::DeepText(trimmed.to_string())
    }
}

/// Simple email validation (contains @ and has content on both sides).
fn is_valid_email(s: &str) -> bool {
    let parts: Vec<&str> = s.split('@').collect();
    parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() && parts[1].contains('.')
}

/// Perform ticket search with basic (query) and advanced (multi-criteria) filtering.
///
/// @implements FS-020.7: basic search (query param).
/// @implements FS-020.8: advanced search (status, deptId, assignee, staffId, topicId, dates).
/// @implements BS-020.6: query must be >= 3 chars.
/// @implements BS-020.2: visibility scoping always applied.
/// @implements BS-020.3: out-of-scope deptId is silently ignored.
/// @implements EC-020.3: invalid date span rejected.
async fn search_tickets(
    pool: &sqlx::postgres::PgPool,
    staff_id: i32,
    params: &QueueParams,
) -> Result<Json<QueueResponse>, ApiError> {
    // BS-020.6: Validate query length (>= 3 chars if provided).
    // @implements FS-020.7 AC-4/AC-5: query < 3 chars or empty returns 400.
    if let Some(ref query) = params.query {
        let trimmed = query.trim();
        if !trimmed.is_empty() && trimmed.len() < 3 {
            return Err(ApiError::bad_request("Search term must be more than 3 chars"));
        }
        if trimmed.is_empty() {
            return Err(ApiError::bad_request("Search term must be more than 3 chars"));
        }
    }

    // EC-020.3: Validate date span if both dates are provided.
    if let (Some(ref start), Some(ref end)) = (&params.start_date, &params.end_date) {
        if !start.is_empty() && !end.is_empty() && start > end {
            return Err(ApiError::bad_request("Entered date span is invalid."));
        }
    }

    // Load visibility context.
    let visibility = load_staff_visibility(pool, staff_id)
        .await
        .map_err(|_| ApiError::internal("Visibility lookup failed"))?;

    // Pagination.
    let page = params.page();
    let page_size = resolve_page_size(pool, staff_id, params.limit()).await;
    let offset = (page.saturating_sub(1)) * page_size;

    // Build the search query.
    let (items, total) = execute_search_query(
        pool,
        &visibility,
        params,
        page_size,
        offset,
    )
    .await?;

    let total_pages = if total == 0 {
        1
    } else {
        ((total as u32 + page_size - 1) / page_size).max(1)
    };

    // EC-020.10: status_column is true when status=any (no status filter).
    let status_column = params.search_status().is_none();

    Ok(Json(QueueResponse {
        tickets: items,
        pagination: PaginationInfo {
            page,
            page_size,
            total_count: total,
            total_pages,
        },
        rightmost_column: "department".to_string(),
        search_results: Some(true),
        status_column: if status_column { Some(true) } else { None },
    }))
}

/// Execute the search query with all criteria applied.
///
/// @implements FS-020.7: basic keyword search.
/// @implements FS-020.8: advanced multi-criteria search.
/// @implements BS-020.2: visibility always enforced.
async fn execute_search_query(
    pool: &sqlx::postgres::PgPool,
    visibility: &StaffVisibility,
    params: &QueueParams,
    page_size: u32,
    offset: u32,
) -> Result<(Vec<QueueItem>, i64), ApiError> {
    let visibility_clause = visibility.build_where_clause();

    // Build WHERE clauses.
    let mut where_clauses: Vec<String> = vec![visibility_clause];
    let mut needs_thread_join = false;

    // Status filter (advanced search).
    // @implements FS-020.8: status filter.
    if let Some(status) = params.search_status() {
        match status {
            "open" => where_clauses.push("ticket.status = 'open' AND ticket.isoverdue = false AND ticket.isanswered = false".to_string()),
            "answered" => where_clauses.push("ticket.status = 'open' AND ticket.isanswered = true".to_string()),
            "overdue" => where_clauses.push("ticket.status = 'open' AND ticket.isoverdue = true".to_string()),
            "closed" => where_clauses.push("ticket.status = 'closed'".to_string()),
            _ => {} // "any" or unknown -> no status filter
        }
    }

    // Department filter (bounded by access).
    // @implements BS-020.3: out-of-scope deptId is silently ignored.
    if let Some(dept_id) = params.dept_id {
        if visibility.dept_ids.contains(&dept_id) {
            where_clauses.push(format!("ticket.dept_id = {}", dept_id));
        }
        // If dept_id is out of scope, silently ignore (don't add a filter).
    }

    // Assignee filter (ignored when status=closed).
    // @implements FS-020.8: assignee filter.
    if let Some(ref assignee) = params.assignee {
        let status_is_closed = params.search_status() == Some("closed");
        if !status_is_closed {
            if let Some(clause) = parse_assignee_clause(assignee) {
                where_clauses.push(clause);
            }
        }
    }

    // Closed-by staff filter.
    // @implements FS-020.8: staffId (closed-by) filter.
    if let Some(staff_id) = params.staff_id {
        if staff_id > 0 {
            where_clauses.push(format!("ticket.closed_by_staff_id = {}", staff_id));
        }
    }

    // Topic filter.
    // @implements FS-020.8: topicId filter.
    if let Some(topic_id) = params.topic_id {
        where_clauses.push(format!("ticket.topic_id = {}", topic_id));
    }

    // Date range filter.
    // @implements FS-020.8: startDate/endDate filter.
    if let Some(ref start_date) = params.start_date {
        if !start_date.is_empty() {
            where_clauses.push(format!("ticket.created >= '{}'::date", escape_sql_string(start_date)));
        }
    }
    if let Some(ref end_date) = params.end_date {
        if !end_date.is_empty() {
            // End date is inclusive, so we add 1 day.
            where_clauses.push(format!("ticket.created < ('{}' ::date + interval '1 day')", escape_sql_string(end_date)));
        }
    }

    // Keyword search.
    // @implements BS-020.7: keyword resolution strategy.
    if let Some(ref query) = params.query {
        let trimmed = query.trim();
        if !trimmed.is_empty() {
            let keyword_type = KeywordType::from_query(trimmed);
            match keyword_type {
                KeywordType::TicketNumber(num) => {
                    // Prefix match on ticket number.
                    where_clauses.push(format!(r#"ticket."ticketID"::text LIKE '{}%'"#, escape_sql_string(&num)));
                }
                KeywordType::Email(email) => {
                    // Exact match on requester email.
                    where_clauses.push(format!("ticket.email = '{}'", escape_sql_string(&email)));
                }
                KeywordType::DeepText(text) => {
                    // Deep search across email, name, subject, and thread body.
                    needs_thread_join = true;
                    let pattern = format!("%{}%", escape_sql_string(&text));
                    where_clauses.push(format!(
                        "(ticket.email ILIKE '{}' OR ticket.name ILIKE '{}' OR ticket.subject ILIKE '{}' OR tt.body ILIKE '{}')",
                        pattern, pattern, pattern, pattern
                    ));
                }
            }
        }
    }

    let where_clause = where_clauses.join(" AND ");

    // Build JOIN clause.
    let thread_join = if needs_thread_join {
        "LEFT JOIN ticket_thread tt ON tt.ticket_id = ticket.ticket_id"
    } else {
        ""
    };

    // Build query with DISTINCT to avoid duplicates from thread join.
    // @implements FS-020.7 AC-6: DISTINCT tickets.
    let select_clause = r#"
        SELECT DISTINCT ON (ticket.ticket_id)
            ticket.ticket_id, ticket."ticketID", ticket.subject, ticket.email,
            to_char(ticket.created, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created,
            ticket.isanswered, ticket.isoverdue,
            to_char(ticket.duedate, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS duedate,
            COALESCE(department.dept_name, '') AS department_name
    "#;

    let query = format!(
        r#"{select_clause}
           FROM ticket
           {thread_join}
           LEFT JOIN department ON department.dept_id = ticket.dept_id
           WHERE {where_clause}
           ORDER BY ticket.ticket_id DESC, ticket.created DESC
           LIMIT {page_size} OFFSET {offset}"#
    );

    let count_query = if needs_thread_join {
        format!(
            "SELECT COUNT(DISTINCT ticket.ticket_id) FROM ticket {} WHERE {}",
            thread_join, where_clause
        )
    } else {
        format!("SELECT COUNT(*) FROM ticket WHERE {}", where_clause)
    };

    let rows: Vec<(i64, i64, String, String, String, bool, bool, Option<String>, String)> =
        sqlx::query_as(&query)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, query = %query, "Search query failed");
                ApiError::internal("Search query failed")
            })?;

    let total: i64 = sqlx::query_scalar(&count_query)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Search count failed");
            ApiError::internal("Search count failed")
        })?;

    let items = rows
        .into_iter()
        .map(|(id, number, subject, email, created, isanswered, isoverdue, duedate, department_name)| {
            QueueItem {
                id,
                number,
                subject,
                email,
                created,
                isanswered,
                isoverdue,
                duedate,
                assigned_to_name: None,
                closed_by_name: None,
                department_name: Some(department_name),
            }
        })
        .collect();

    Ok((items, total))
}

/// Parse the assignee filter into a SQL clause.
/// Format: s{id} for staff, t{id} for team, s0 for unassigned.
///
/// @implements FS-020.8: assignee filter parsing.
fn parse_assignee_clause(assignee: &str) -> Option<String> {
    let trimmed = assignee.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed == "s0" {
        // Unassigned: staff_id IS NULL and team_id is 0 (the "no team" sentinel).
        // Note: team_id column is NOT NULL with default 0, so we check = 0 not IS NULL.
        return Some("ticket.staff_id IS NULL AND ticket.team_id = 0".to_string());
    }

    if let Some(id_str) = trimmed.strip_prefix('s') {
        // Staff assignment.
        if let Ok(id) = id_str.parse::<i32>() {
            return Some(format!("ticket.staff_id = {}", id));
        }
    }

    if let Some(id_str) = trimmed.strip_prefix('t') {
        // Team assignment.
        if let Ok(id) = id_str.parse::<i32>() {
            return Some(format!("ticket.team_id = {}", id));
        }
    }

    // Plain numeric -> treat as staff id.
    if let Ok(id) = trimmed.parse::<i32>() {
        return Some(format!("ticket.staff_id = {}", id));
    }

    None
}

/// Escape a string for use in SQL (simple apostrophe escaping).
fn escape_sql_string(s: &str) -> String {
    s.replace('\'', "''")
}

/// Resolve page size per FS-090.19.
///
/// Priority: limit param (already clamped) > staff personal > system default > 25.
///
/// @implements FS-090.19: page-size resolution.
async fn resolve_page_size(
    pool: &sqlx::postgres::PgPool,
    staff_id: i32,
    limit_param: Option<u32>,
) -> u32 {
    const DEFAULT_PAGE_SIZE: u32 = 25;

    // 1. If limit param is present (already clamped by QueueParams::limit), use it.
    if let Some(limit) = limit_param {
        return limit;
    }

    // 2. Check staff personal page size (FS-031.10 Maximum Page Size preference).
    //    The column is `max_page_size` (M4-PREP additive column); the pre-M4 read
    //    used a non-existent `page_limit`, whose error was silently swallowed by
    //    `.ok().flatten()` — repointed here so the personal override actually
    //    takes effect (TS-M4-B1).
    let staff_limit: Option<i32> =
        sqlx::query_scalar("SELECT max_page_size FROM staff WHERE staff_id = $1")
            .bind(staff_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    if let Some(limit) = staff_limit {
        if limit > 0 {
            return (limit as u32).clamp(5, 100);
        }
    }

    // 3. Check system default_page_size config.
    let system_limit: Option<String> =
        sqlx::query_scalar("SELECT value FROM config WHERE key = 'default_page_size'")
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    if let Some(val) = system_limit {
        if let Ok(limit) = val.parse::<u32>() {
            return limit.clamp(5, 100);
        }
    }

    // 4. Fall back to default 25.
    DEFAULT_PAGE_SIZE
}

/// Resolve the effective sort for a queue request.
///
/// Priority: explicit valid sort param > session sticky pref > queue default.
///
/// Returns (effective_sort_config, should_store_in_session).
///
/// @implements BS-020.8: sticky per-queue sort preferences resolution.
fn resolve_effective_sort(
    explicit: &SortConfig,
    queue: QueueStatus,
    session_prefs: Option<&SortPreferences>,
    params: &QueueParams,
) -> (SortConfig, bool) {
    // If explicit sort key is provided and valid, use it and mark for storage.
    if explicit.key.is_some() {
        return (explicit.clone(), true);
    }

    // If sort param was provided but invalid, don't store and use default.
    if params.sort.is_some() {
        // Invalid sort key was provided — don't store, use queue default.
        return (SortConfig { key: None, order: SortOrder::Desc }, false);
    }

    // No explicit sort param — check session for sticky preference.
    if let Some(prefs) = session_prefs {
        if let Some(queue_sort) = get_queue_sort(prefs, queue) {
            if let Some(key) = SortKey::from_str(&queue_sort.sort) {
                let order = SortOrder::from_str(&queue_sort.order);
                return (SortConfig { key: Some(key), order }, false);
            }
        }
    }

    // No explicit, no session pref — use queue default.
    (SortConfig { key: None, order: SortOrder::Desc }, false)
}

/// Get the sort preference for a specific queue from session prefs.
fn get_queue_sort(prefs: &SortPreferences, queue: QueueStatus) -> Option<&QueueSort> {
    match queue {
        QueueStatus::Open => prefs.open.as_ref(),
        QueueStatus::Answered => prefs.answered.as_ref(),
        QueueStatus::Assigned => prefs.assigned.as_ref(),
        QueueStatus::Overdue => prefs.overdue.as_ref(),
        QueueStatus::Closed => prefs.closed.as_ref(),
    }
}

/// Store a sort preference in the session.
///
/// @implements BS-020.8: persist sticky sort preference.
async fn store_sort_preference(
    store: &ost_core::session::SessionStore,
    session_id: &str,
    staff_id: i32,
    current_prefs: Option<SortPreferences>,
    queue: QueueStatus,
    sort_config: &SortConfig,
) {
    // Only store if we have a valid sort key.
    let Some(key) = sort_config.key else {
        return;
    };

    let mut prefs = current_prefs.unwrap_or_default();
    let new_sort = QueueSort {
        sort: sort_key_to_string(key),
        order: sort_config.order.sql().to_string(),
    };

    // Update the appropriate queue's sort pref.
    match queue {
        QueueStatus::Open => prefs.open = Some(new_sort),
        QueueStatus::Answered => prefs.answered = Some(new_sort),
        QueueStatus::Assigned => prefs.assigned = Some(new_sort),
        QueueStatus::Overdue => prefs.overdue = Some(new_sort),
        QueueStatus::Closed => prefs.closed = Some(new_sort),
    }

    // Update the session.
    let data = SessionData::Staff {
        staff_id,
        sort_prefs: Some(prefs),
    };
    let _ = store.update(session_id, &data).await;
}

/// Convert a SortKey back to its string form for storage.
fn sort_key_to_string(key: SortKey) -> String {
    match key {
        SortKey::Date => "date",
        SortKey::Id => "ID",
        SortKey::Priority => "pri",
        SortKey::Name => "name",
        SortKey::Subject => "subj",
        SortKey::Status => "status",
        SortKey::Assignee => "assignee",
        SortKey::Staff => "staff",
        SortKey::Department => "dept",
    }
    .to_string()
}

/// Per-queue default sort expressions.
///
/// @implements FS-020.5: default sort by queue.
fn default_order_by(queue: QueueStatus) -> &'static str {
    match queue {
        // Overdue: priority urgency ASC, due_date ASC NULLS LAST, effective_date ASC, created ASC
        QueueStatus::Overdue => {
            "COALESCE(priority.urgency, 2) ASC, \
             ticket.duedate ASC NULLS LAST, \
             COALESCE(ticket.reopened, ticket.lastmessage, ticket.created) ASC, \
             ticket.created ASC"
        }
        // Closed: closed_date DESC, created DESC
        QueueStatus::Closed => {
            "ticket.closed DESC NULLS LAST, ticket.created DESC"
        }
        // Answered: last_response_date DESC, created DESC
        QueueStatus::Answered => {
            "ticket.lastresponse DESC NULLS LAST, ticket.created DESC"
        }
        // All other open queues: priority urgency ASC, effective_date DESC, created DESC
        QueueStatus::Open | QueueStatus::Assigned => {
            "COALESCE(priority.urgency, 2) ASC, \
             COALESCE(ticket.reopened, ticket.lastmessage, ticket.created) DESC, \
             ticket.created DESC"
        }
    }
}

/// Resolve the ORDER BY clause based on sort config and queue defaults.
///
/// @implements FS-020.5: sort param handling + per-queue default sorts.
fn resolve_order_by(queue: QueueStatus, sort_config: &SortConfig) -> String {
    match sort_config.key {
        Some(key) => {
            // Explicit sort key provided
            format!(
                "{} {}, ticket.ticket_id {}",
                key.sql_expression(),
                sort_config.order.sql(),
                sort_config.order.sql()
            )
        }
        None => {
            // Use per-queue default sort
            default_order_by(queue).to_string()
        }
    }
}

/// Check if an ORDER BY clause requires a priority JOIN.
fn order_needs_priority_join(order_by: &str) -> bool {
    order_by.contains("priority.urgency")
}

/// Check if an ORDER BY clause requires a staff JOIN (for sort=assignee).
///
/// @implements FS-020.5: sort=assignee requires staff JOIN.
fn order_needs_staff_join(order_by: &str) -> bool {
    order_by.contains("staff.firstname")
}

/// Check if an ORDER BY clause requires a department JOIN (for sort=dept).
///
/// @implements FS-020.5: sort=dept requires department JOIN.
fn order_needs_dept_join(order_by: &str) -> bool {
    order_by.contains("department.dept_name")
}

/// Check if an ORDER BY clause requires a closing_staff JOIN (for sort=staff).
///
/// @implements FS-020.5: sort=staff requires closing_staff JOIN.
fn order_needs_closing_staff_join(order_by: &str) -> bool {
    order_by.contains("closing_staff.firstname")
}

/// Read a config value as a boolean (1/true = true, else false).
async fn read_config_bool(pool: &sqlx::postgres::PgPool, key: &str) -> bool {
    let value: Option<String> =
        sqlx::query_scalar("SELECT value FROM config WHERE key = $1")
            .bind(key)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

    matches!(value.as_deref(), Some("1") | Some("true"))
}

/// Query the open queue with visibility filtering, sorting, and pagination.
///
/// Applies the visibility predicate (staff can only see tickets in their
/// accessible departments, assigned to them, or assigned to their teams).
/// Additionally:
/// - When show_assigned_tickets=0, exclude tickets assigned to anyone.
/// - When show_answered_tickets=0, exclude answered tickets.
///
/// @implements BS-020.1: open queue with visibility rules.
/// @implements BS-020.2: visibility scoping (dept + assignment predicate).
/// @implements BS-020.4: answered/assigned visibility rules.
/// @implements FS-020.5: sorting with explicit/default ORDER BY.
/// @implements FS-020.6: pagination with LIMIT/OFFSET.
/// @implements BS-020.5: fetch rightmost column data (assigned_to or department).
async fn query_open_queue_with_visibility(
    pool: &sqlx::postgres::PgPool,
    show_assigned: bool,
    show_answered: bool,
    visibility: &StaffVisibility,
    order_by: &str,
    page: u32,
    page_size: u32,
    rightmost_column: RightmostColumn,
) -> Result<(Vec<QueueItem>, i64), ApiError> {
    // Build WHERE clause dynamically based on visibility + config toggles.
    // Base: status='open' AND isoverdue=false AND visibility predicate
    // Note: All columns qualified with `ticket.` to avoid ambiguity with JOINs.
    let visibility_clause = visibility.build_where_clause();
    let mut where_clause = format!(
        "ticket.status = 'open' AND ticket.isoverdue = false AND {}",
        visibility_clause
    );

    if !show_assigned {
        where_clause.push_str(" AND ticket.staff_id IS NULL");
    }
    if !show_answered {
        where_clause.push_str(" AND ticket.isanswered = false");
    }

    // Build JOINs based on what the ORDER BY needs and the rightmost column.
    // @implements BS-020.5: join staff/team/department for rightmost column.
    // @implements FS-020.5: additional JOINs for sort keys (assignee, dept, staff).
    let mut joins = String::new();
    if order_needs_priority_join(order_by) {
        joins.push_str("LEFT JOIN priority ON priority.priority_id = ticket.priority_id ");
    }
    // JOINs for sort keys (sort=assignee, sort=dept, sort=staff).
    // These use unaliased table names to match SortKey::sql_expression().
    if order_needs_staff_join(order_by) {
        joins.push_str("LEFT JOIN staff ON staff.staff_id = ticket.staff_id ");
    }
    if order_needs_dept_join(order_by) && rightmost_column != RightmostColumn::Department {
        // Only add if not already joined for rightmost_column=department
        joins.push_str("LEFT JOIN department ON department.dept_id = ticket.dept_id ");
    }
    if order_needs_closing_staff_join(order_by) {
        joins.push_str("LEFT JOIN staff AS closing_staff ON closing_staff.staff_id = ticket.closed_by_staff_id ");
    }

    // Additional columns and joins based on rightmost column.
    let (extra_columns, extra_joins) = match rightmost_column {
        RightmostColumn::AssignedTo => (
            ", COALESCE(NULLIF(TRIM(CONCAT(assigned_staff.firstname, ' ', assigned_staff.lastname)), ''), assigned_staff.username, team.name, '') AS assigned_to_name",
            "LEFT JOIN staff AS assigned_staff ON assigned_staff.staff_id = ticket.staff_id \
             LEFT JOIN team ON team.team_id = ticket.team_id"
        ),
        RightmostColumn::Department => (
            ", COALESCE(department.dept_name, '') AS department_name",
            "LEFT JOIN department ON department.dept_id = ticket.dept_id"
        ),
        RightmostColumn::ClosedBy => (
            ", COALESCE(NULLIF(TRIM(CONCAT(closing_staff.firstname, ' ', closing_staff.lastname)), ''), closing_staff.username, '') AS closed_by_name",
            "LEFT JOIN staff AS closing_staff ON closing_staff.staff_id = ticket.closed_by_staff_id"
        ),
    };
    joins.push_str(extra_joins);

    // Calculate OFFSET for pagination.
    let offset = (page.saturating_sub(1)) * page_size;

    // @implements BS-032.10: include isoverdue and duedate in listing.
    let query = format!(
        r#"SELECT ticket.ticket_id, ticket."ticketID", ticket.subject, ticket.email,
                  to_char(ticket.created, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created,
                  ticket.isanswered,
                  ticket.isoverdue,
                  to_char(ticket.duedate, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS duedate
                  {extra_columns}
           FROM ticket
           {joins}
           WHERE {where_clause}
           ORDER BY {order_by}
           LIMIT {page_size} OFFSET {offset}"#
    );

    // Count query: need to re-build where_clause without JOINs or use the same
    // qualified columns (which is safe for single-table queries).
    let count_query = format!("SELECT COUNT(*) FROM ticket WHERE {where_clause}");

    // The row type now includes isoverdue, duedate, plus the rightmost extra column.
    let rows: Vec<(i64, i64, String, String, String, bool, bool, Option<String>, String)> = sqlx::query_as(&query)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, query = %query, "Ticket queue lookup failed");
            ApiError::internal("Ticket queue lookup failed")
        })?;

    let total: i64 = sqlx::query_scalar(&count_query)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal("Ticket count failed"))?;

    let items = rows
        .into_iter()
        .map(|(id, number, subject, email, created, isanswered, isoverdue, duedate, extra)| {
            let (assigned_to_name, closed_by_name, department_name) = match rightmost_column {
                RightmostColumn::AssignedTo => (Some(extra), None, None),
                RightmostColumn::ClosedBy => (None, Some(extra), None),
                RightmostColumn::Department => (None, None, Some(extra)),
            };
            QueueItem {
                id,
                number,
                subject,
                email,
                created,
                isanswered,
                isoverdue,
                duedate,
                assigned_to_name,
                closed_by_name,
                department_name,
            }
        })
        .collect();

    Ok((items, total))
}

/// Query the answered queue with visibility, sorting, and pagination.
///
/// @implements BS-020.1: answered queue.
/// @implements BS-020.2: visibility scoping.
/// @implements FS-020.5: sorting with explicit/default ORDER BY.
/// @implements FS-020.6: pagination with LIMIT/OFFSET.
/// @implements BS-020.5: fetch rightmost column data (assigned_to).
async fn query_answered_queue_with_visibility(
    pool: &sqlx::postgres::PgPool,
    visibility: &StaffVisibility,
    order_by: &str,
    page: u32,
    page_size: u32,
    rightmost_column: RightmostColumn,
) -> Result<(Vec<QueueItem>, i64), ApiError> {
    let visibility_clause = visibility.build_where_clause();
    let where_clause = format!(
        "ticket.status = 'open' AND ticket.isanswered = true AND {}",
        visibility_clause
    );

    // Build JOINs based on what the ORDER BY needs and the rightmost column.
    // @implements FS-020.5: additional JOINs for sort keys (assignee, dept, staff).
    let mut joins = String::new();
    if order_needs_priority_join(order_by) {
        joins.push_str("LEFT JOIN priority ON priority.priority_id = ticket.priority_id ");
    }
    // JOINs for sort keys (sort=assignee, sort=dept, sort=staff).
    if order_needs_staff_join(order_by) {
        joins.push_str("LEFT JOIN staff ON staff.staff_id = ticket.staff_id ");
    }
    if order_needs_dept_join(order_by) && rightmost_column != RightmostColumn::Department {
        joins.push_str("LEFT JOIN department ON department.dept_id = ticket.dept_id ");
    }
    if order_needs_closing_staff_join(order_by) {
        joins.push_str("LEFT JOIN staff AS closing_staff ON closing_staff.staff_id = ticket.closed_by_staff_id ");
    }

    // Additional columns and joins based on rightmost column (always assigned_to for answered).
    let (extra_columns, extra_joins) = match rightmost_column {
        RightmostColumn::AssignedTo => (
            ", COALESCE(NULLIF(TRIM(CONCAT(assigned_staff.firstname, ' ', assigned_staff.lastname)), ''), assigned_staff.username, team.name, '') AS assigned_to_name",
            "LEFT JOIN staff AS assigned_staff ON assigned_staff.staff_id = ticket.staff_id \
             LEFT JOIN team ON team.team_id = ticket.team_id"
        ),
        RightmostColumn::Department => (
            ", COALESCE(department.dept_name, '') AS department_name",
            "LEFT JOIN department ON department.dept_id = ticket.dept_id"
        ),
        RightmostColumn::ClosedBy => (
            ", COALESCE(NULLIF(TRIM(CONCAT(closing_staff.firstname, ' ', closing_staff.lastname)), ''), closing_staff.username, '') AS closed_by_name",
            "LEFT JOIN staff AS closing_staff ON closing_staff.staff_id = ticket.closed_by_staff_id"
        ),
    };
    joins.push_str(extra_joins);

    let offset = (page.saturating_sub(1)) * page_size;

    // @implements BS-032.10: include isoverdue and duedate in listing.
    let query = format!(
        r#"SELECT ticket.ticket_id, ticket."ticketID", ticket.subject, ticket.email,
                  to_char(ticket.created, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created,
                  ticket.isanswered,
                  ticket.isoverdue,
                  to_char(ticket.duedate, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS duedate
                  {extra_columns}
           FROM ticket
           {joins}
           WHERE {where_clause}
           ORDER BY {order_by}
           LIMIT {page_size} OFFSET {offset}"#
    );

    let count_query = format!("SELECT COUNT(*) FROM ticket WHERE {where_clause}");

    let rows: Vec<(i64, i64, String, String, String, bool, bool, Option<String>, String)> = sqlx::query_as(&query)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Ticket queue lookup failed");
            ApiError::internal("Ticket queue lookup failed")
        })?;

    let total: i64 = sqlx::query_scalar(&count_query)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal("Ticket count failed"))?;

    let items = rows
        .into_iter()
        .map(|(id, number, subject, email, created, isanswered, isoverdue, duedate, extra)| {
            let (assigned_to_name, closed_by_name, department_name) = match rightmost_column {
                RightmostColumn::AssignedTo => (Some(extra), None, None),
                RightmostColumn::ClosedBy => (None, Some(extra), None),
                RightmostColumn::Department => (None, None, Some(extra)),
            };
            QueueItem {
                id,
                number,
                subject,
                email,
                created,
                isanswered,
                isoverdue,
                duedate,
                assigned_to_name,
                closed_by_name,
                department_name,
            }
        })
        .collect();

    Ok((items, total))
}

/// Query the assigned queue with sorting and pagination.
///
/// The assigned queue ("My Tickets") always shows department as the rightmost
/// column, since "Assigned To" would always be the current user ("me").
///
/// @implements BS-020.1: assigned queue (My Tickets).
/// @implements FS-020.5: sorting with explicit/default ORDER BY.
/// @implements FS-020.6: pagination with LIMIT/OFFSET.
/// @implements BS-020.5: rightmost column is always department for assigned queue.
async fn query_assigned_queue(
    pool: &sqlx::postgres::PgPool,
    staff_id: i32,
    order_by: &str,
    page: u32,
    page_size: u32,
) -> Result<(Vec<QueueItem>, i64), ApiError> {
    // Build JOINs based on what the ORDER BY needs + department for rightmost column.
    // @implements FS-020.5: additional JOINs for sort keys (assignee, dept, staff).
    let mut joins = String::new();
    if order_needs_priority_join(order_by) {
        joins.push_str("LEFT JOIN priority ON priority.priority_id = ticket.priority_id ");
    }
    // JOINs for sort keys (sort=assignee, sort=staff).
    // Note: department is already joined for rightmost column, so skip dept join check here.
    if order_needs_staff_join(order_by) {
        joins.push_str("LEFT JOIN staff ON staff.staff_id = ticket.staff_id ");
    }
    if order_needs_closing_staff_join(order_by) {
        joins.push_str("LEFT JOIN staff AS closing_staff ON closing_staff.staff_id = ticket.closed_by_staff_id ");
    }
    // Always join department for assigned queue's rightmost column.
    joins.push_str("LEFT JOIN department ON department.dept_id = ticket.dept_id ");

    let offset = (page.saturating_sub(1)) * page_size;

    // @implements BS-032.10: include isoverdue and duedate in listing.
    let query = format!(
        r#"SELECT ticket.ticket_id, ticket."ticketID", ticket.subject, ticket.email,
                  to_char(ticket.created, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created,
                  ticket.isanswered,
                  ticket.isoverdue,
                  to_char(ticket.duedate, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS duedate,
                  COALESCE(department.dept_name, '') AS department_name
           FROM ticket
           {joins}
           WHERE status = 'open' AND ticket.staff_id = $1
           ORDER BY {order_by}
           LIMIT {page_size} OFFSET {offset}"#
    );

    let rows: Vec<(i64, i64, String, String, String, bool, bool, Option<String>, String)> = sqlx::query_as(&query)
        .bind(staff_id)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Ticket queue lookup failed");
            ApiError::internal("Ticket queue lookup failed")
        })?;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ticket WHERE status = 'open' AND staff_id = $1",
    )
    .bind(staff_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal("Ticket count failed"))?;

    let items = rows
        .into_iter()
        .map(|(id, number, subject, email, created, isanswered, isoverdue, duedate, department_name)| QueueItem {
            id,
            number,
            subject,
            email,
            created,
            isanswered,
            isoverdue,
            duedate,
            assigned_to_name: None,
            closed_by_name: None,
            department_name: Some(department_name),
        })
        .collect();

    Ok((items, total))
}

/// Query the overdue queue with visibility, sorting, and pagination.
///
/// @implements BS-020.1: overdue queue.
/// @implements BS-020.2: visibility scoping.
/// @implements FS-020.5: sorting with explicit/default ORDER BY.
/// @implements FS-020.6: pagination with LIMIT/OFFSET.
/// @implements BS-020.5: fetch rightmost column data (assigned_to).
async fn query_overdue_queue_with_visibility(
    pool: &sqlx::postgres::PgPool,
    visibility: &StaffVisibility,
    order_by: &str,
    page: u32,
    page_size: u32,
    rightmost_column: RightmostColumn,
) -> Result<(Vec<QueueItem>, i64), ApiError> {
    let visibility_clause = visibility.build_where_clause();
    let where_clause = format!(
        "ticket.status = 'open' AND ticket.isoverdue = true AND {}",
        visibility_clause
    );

    // Build JOINs based on what the ORDER BY needs and the rightmost column.
    // @implements FS-020.5: additional JOINs for sort keys (assignee, dept, staff).
    let mut joins = String::new();
    if order_needs_priority_join(order_by) {
        joins.push_str("LEFT JOIN priority ON priority.priority_id = ticket.priority_id ");
    }
    // JOINs for sort keys (sort=assignee, sort=dept, sort=staff).
    if order_needs_staff_join(order_by) {
        joins.push_str("LEFT JOIN staff ON staff.staff_id = ticket.staff_id ");
    }
    if order_needs_dept_join(order_by) && rightmost_column != RightmostColumn::Department {
        joins.push_str("LEFT JOIN department ON department.dept_id = ticket.dept_id ");
    }
    if order_needs_closing_staff_join(order_by) {
        joins.push_str("LEFT JOIN staff AS closing_staff ON closing_staff.staff_id = ticket.closed_by_staff_id ");
    }

    // Additional columns and joins based on rightmost column (always assigned_to for overdue).
    let (extra_columns, extra_joins) = match rightmost_column {
        RightmostColumn::AssignedTo => (
            ", COALESCE(NULLIF(TRIM(CONCAT(assigned_staff.firstname, ' ', assigned_staff.lastname)), ''), assigned_staff.username, team.name, '') AS assigned_to_name",
            "LEFT JOIN staff AS assigned_staff ON assigned_staff.staff_id = ticket.staff_id \
             LEFT JOIN team ON team.team_id = ticket.team_id"
        ),
        RightmostColumn::Department => (
            ", COALESCE(department.dept_name, '') AS department_name",
            "LEFT JOIN department ON department.dept_id = ticket.dept_id"
        ),
        RightmostColumn::ClosedBy => (
            ", COALESCE(NULLIF(TRIM(CONCAT(closing_staff.firstname, ' ', closing_staff.lastname)), ''), closing_staff.username, '') AS closed_by_name",
            "LEFT JOIN staff AS closing_staff ON closing_staff.staff_id = ticket.closed_by_staff_id"
        ),
    };
    joins.push_str(extra_joins);

    let offset = (page.saturating_sub(1)) * page_size;

    // @implements BS-032.10: include isoverdue and duedate in listing.
    let query = format!(
        r#"SELECT ticket.ticket_id, ticket."ticketID", ticket.subject, ticket.email,
                  to_char(ticket.created, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created,
                  ticket.isanswered,
                  ticket.isoverdue,
                  to_char(ticket.duedate, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS duedate
                  {extra_columns}
           FROM ticket
           {joins}
           WHERE {where_clause}
           ORDER BY {order_by}
           LIMIT {page_size} OFFSET {offset}"#
    );

    let count_query = format!("SELECT COUNT(*) FROM ticket WHERE {where_clause}");

    let rows: Vec<(i64, i64, String, String, String, bool, bool, Option<String>, String)> = sqlx::query_as(&query)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Ticket queue lookup failed");
            ApiError::internal("Ticket queue lookup failed")
        })?;

    let total: i64 = sqlx::query_scalar(&count_query)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal("Ticket count failed"))?;

    let items = rows
        .into_iter()
        .map(|(id, number, subject, email, created, isanswered, isoverdue, duedate, extra)| {
            let (assigned_to_name, closed_by_name, department_name) = match rightmost_column {
                RightmostColumn::AssignedTo => (Some(extra), None, None),
                RightmostColumn::ClosedBy => (None, Some(extra), None),
                RightmostColumn::Department => (None, None, Some(extra)),
            };
            QueueItem {
                id,
                number,
                subject,
                email,
                created,
                isanswered,
                isoverdue,
                duedate,
                assigned_to_name,
                closed_by_name,
                department_name,
            }
        })
        .collect();

    Ok((items, total))
}

/// Query the closed queue with visibility, sorting, and pagination.
///
/// For closed tickets, the visibility predicate is modified: staff can see
/// closed tickets in their accessible departments (not requiring status='open'
/// on the visibility clause). Direct assignment and team assignment still apply.
///
/// The closed queue always shows "Closed By" as the rightmost column.
///
/// @implements BS-020.1: closed queue.
/// @implements BS-020.2: visibility scoping for closed tickets.
/// @implements FS-020.5: sorting with explicit/default ORDER BY.
/// @implements FS-020.6: pagination with LIMIT/OFFSET.
/// @implements BS-020.5: rightmost column is always closed_by for closed queue.
async fn query_closed_queue_with_visibility(
    pool: &sqlx::postgres::PgPool,
    visibility: &StaffVisibility,
    order_by: &str,
    page: u32,
    page_size: u32,
) -> Result<(Vec<QueueItem>, i64), ApiError> {
    // For closed queue, we need a modified visibility predicate that doesn't
    // require status='open'. The predicate becomes:
    // - Tickets assigned to me (regardless of status)
    // - Tickets in my accessible departments (regardless of status)
    // - Tickets assigned to my teams (regardless of status)
    let visibility_clause = build_closed_visibility_clause(visibility);
    let where_clause = format!("ticket.status = 'closed' AND {}", visibility_clause);

    // Build JOINs based on what the ORDER BY needs + closing staff for rightmost column.
    // @implements FS-020.5: additional JOINs for sort keys (assignee, dept, staff).
    let mut joins = String::new();
    if order_needs_priority_join(order_by) {
        joins.push_str("LEFT JOIN priority ON priority.priority_id = ticket.priority_id ");
    }
    // JOINs for sort keys (sort=assignee, sort=dept).
    // Note: closing_staff is already joined for rightmost column.
    if order_needs_staff_join(order_by) {
        joins.push_str("LEFT JOIN staff ON staff.staff_id = ticket.staff_id ");
    }
    if order_needs_dept_join(order_by) {
        joins.push_str("LEFT JOIN department ON department.dept_id = ticket.dept_id ");
    }
    // Always join closing staff for closed queue's rightmost column.
    joins.push_str("LEFT JOIN staff AS closing_staff ON closing_staff.staff_id = ticket.closed_by_staff_id ");

    let offset = (page.saturating_sub(1)) * page_size;

    // @implements BS-032.10: include isoverdue and duedate in listing.
    // Note: Closed tickets have isoverdue=false and duedate=NULL per BS-021.4.
    let query = format!(
        r#"SELECT ticket.ticket_id, ticket."ticketID", ticket.subject, ticket.email,
                  to_char(ticket.created, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created,
                  ticket.isanswered,
                  ticket.isoverdue,
                  to_char(ticket.duedate, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS duedate,
                  COALESCE(NULLIF(TRIM(CONCAT(closing_staff.firstname, ' ', closing_staff.lastname)), ''), closing_staff.username, '') AS closed_by_name
           FROM ticket
           {joins}
           WHERE {where_clause}
           ORDER BY {order_by}
           LIMIT {page_size} OFFSET {offset}"#
    );

    let count_query = format!("SELECT COUNT(*) FROM ticket WHERE {where_clause}");

    let rows: Vec<(i64, i64, String, String, String, bool, bool, Option<String>, String)> = sqlx::query_as(&query)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Ticket queue lookup failed");
            ApiError::internal("Ticket queue lookup failed")
        })?;

    let total: i64 = sqlx::query_scalar(&count_query)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal("Ticket count failed"))?;

    let items = rows
        .into_iter()
        .map(|(id, number, subject, email, created, isanswered, isoverdue, duedate, closed_by_name)| QueueItem {
            id,
            number,
            subject,
            email,
            created,
            isanswered,
            isoverdue,
            duedate,
            assigned_to_name: None,
            closed_by_name: Some(closed_by_name),
            department_name: None,
        })
        .collect();

    Ok((items, total))
}

/// Build visibility clause for closed tickets (no status='open' requirement).
///
/// Note: All columns qualified with `ticket.` to avoid ambiguity with JOINs.
///
/// @implements BS-020.2: visibility for closed tickets.
fn build_closed_visibility_clause(visibility: &StaffVisibility) -> String {
    let mut clauses: Vec<String> = Vec::new();

    // Clause 1: Tickets directly assigned to the staff member (any status).
    clauses.push(format!("(ticket.staff_id = {})", visibility.staff_id));

    // Clause 2: Tickets in accessible departments (only if NOT access-limited).
    if !visibility.is_access_limited && !visibility.dept_ids.is_empty() {
        let dept_list = visibility
            .dept_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        clauses.push(format!("(ticket.dept_id IN ({}))", dept_list));
    }

    // Clause 3: Tickets assigned to the staff member's teams (any status).
    if !visibility.team_ids.is_empty() {
        let team_list = visibility
            .team_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        clauses.push(format!("(ticket.team_id IN ({}))", team_list));
    }

    format!("({})", clauses.join(" OR "))
}

/// `GET /api/staff/tickets/stats` — return quick counts per queue.
///
/// Returns counts for each queue status: open, answered, overdue, assigned, closed.
/// The counts are scoped to the staff member's visibility (FS-020.11, BS-020.2).
///
/// @implements FS-020.11: quick-stats endpoint.
/// @implements TS-M3-A2 AC-6: visibility predicate applied to quick stats.
pub async fn get_stats(
    State(state): State<AppState>,
    session: StaffSession,
) -> Result<Json<StaffStats>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Stats lookup is unavailable"))?;

    // Load the staff member's visibility context (BS-020.2).
    let visibility = load_staff_visibility(pool, session.staff_id)
        .await
        .map_err(|_| ApiError::internal("Visibility lookup failed"))?;

    let visibility_clause = visibility.build_where_clause();
    let closed_visibility_clause = build_closed_visibility_clause(&visibility);

    // Count for each queue type, all scoped by visibility.
    // open: status='open' AND NOT isanswered AND NOT isoverdue AND staff_id IS NULL AND visibility
    // (pure open, excluding answered/overdue/assigned which have their own tabs)
    let open_query = format!(
        "SELECT COUNT(*) FROM ticket WHERE status = 'open' AND isanswered = false AND isoverdue = false AND staff_id IS NULL AND {}",
        visibility_clause
    );
    let open: i64 = sqlx::query_scalar(&open_query)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal("Stats lookup failed"))?;

    // answered: status='open' AND isanswered=true AND visibility
    let answered_query = format!(
        "SELECT COUNT(*) FROM ticket WHERE status = 'open' AND isanswered = true AND {}",
        visibility_clause
    );
    let answered: i64 = sqlx::query_scalar(&answered_query)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal("Stats lookup failed"))?;

    // overdue: status='open' AND isoverdue=true AND visibility
    let overdue_query = format!(
        "SELECT COUNT(*) FROM ticket WHERE status = 'open' AND isoverdue = true AND {}",
        visibility_clause
    );
    let overdue: i64 = sqlx::query_scalar(&overdue_query)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal("Stats lookup failed"))?;

    // assigned: status='open' AND staff_id=me (no additional visibility needed,
    // since tickets assigned to me are always visible per BS-020.2 clause 1)
    let assigned: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ticket WHERE status = 'open' AND staff_id = $1",
    )
    .bind(session.staff_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal("Stats lookup failed"))?;

    // closed: status='closed' AND visibility (using closed visibility clause)
    let closed_query = format!(
        "SELECT COUNT(*) FROM ticket WHERE status = 'closed' AND {}",
        closed_visibility_clause
    );
    let closed: i64 = sqlx::query_scalar(&closed_query)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal("Stats lookup failed"))?;

    // Read visibility config for the frontend tab logic (BS-020.4, TS-M3-A4).
    let show_answered_tickets = read_config_bool(pool, "show_answered_tickets").await;
    let show_assigned_tickets = read_config_bool(pool, "show_assigned_tickets").await;

    Ok(Json(StaffStats {
        open,
        answered,
        overdue,
        assigned,
        closed,
        show_answered_tickets,
        show_assigned_tickets,
    }))
}

/// `GET /api/staff/tickets/sort-prefs` — return the staff's sticky sort preferences.
///
/// Returns the current per-queue sort preferences stored in the session. Queues
/// without a stored preference are absent from the response.
///
/// @implements BS-020.8: sort preferences endpoint.
pub async fn get_sort_prefs(
    session: StaffSession,
) -> Json<SortPreferences> {
    Json(session.sort_prefs.unwrap_or_default())
}

/// One thread entry as returned in the staff ticket detail.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadEntryView {
    pub id: i64,
    /// `M` (client message) / `R` (staff response) / `N` (internal note).
    pub thread_type: String,
    pub poster: String,
    /// Optional title (typically used for internal notes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub body: String,
    /// When this entry was created — RFC3339 (`YYYY-MM-DDThh:mm:ssZ`), matching
    /// the ticket-level `created` formatting. Drives the per-entry date in the
    /// staff ticket view.
    pub created: String,
    /// Attachments bound to this entry — `[{id, name, size, mime}]` (§7), empty
    /// when none. The `id` is the `ticket_attachment` binding id (the download
    /// route's `attachmentId`).
    pub attachments: Vec<AttachmentView>,
}

/// A single staff ticket detail: the ticket header + its full thread.
///
/// @implements BS-032.10: includes duedate, sla_id, sla_name.
/// @implements TS-M3-C4: includes staff_id for assignment check.
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
    /// Whether the ticket is overdue.
    ///
    /// @implements BS-032.10: isoverdue flag in detail response.
    pub isoverdue: bool,
    /// Due date (ISO 8601 timestamp), null if no SLA or no explicit due date.
    ///
    /// @implements BS-032.10: duedate field in detail response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duedate: Option<String>,
    /// SLA plan id, null if no SLA.
    ///
    /// @implements BS-032.10: sla_id field in detail response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sla_id: Option<i32>,
    /// SLA plan name, null if no SLA.
    ///
    /// @implements BS-032.10: sla_name field in detail response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sla_name: Option<String>,
    /// Staff id assigned to the ticket, null if unassigned.
    ///
    /// @implements TS-M3-C4: staff_id field for frontend claim button logic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staff_id: Option<i32>,
    /// Collaborative edit-lock state for the requesting agent. Drives the
    /// "This ticket is currently locked by <name>" banner and blocks the reply
    /// composer when another agent holds a live lock. The nested object keeps
    /// snake_case field names (`locked_by_other` / `locked_by_name` /
    /// `expires_at`) — the agreed frontend contract — regardless of the
    /// camelCase envelope.
    ///
    /// @implements FS-021.18: expose lock state on ticket-detail load.
    /// @implements FS-021.20: named "locked by another staff" signal.
    pub lock: ost_core::LockInfo,
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
/// @implements BS-032.10: includes duedate, sla_id, sla_name in response.
pub async fn ticket_detail(
    State(state): State<AppState>,
    session: StaffSession,
    Path(id): Path<i64>,
) -> Result<Json<TicketDetail>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Ticket detail is unavailable"))?;

    let header: Option<TicketHeader> =
        sqlx::query_as(
            r#"SELECT ticket.ticket_id, ticket."ticketID", ticket.subject, ticket.email, ticket.name, ticket.status,
                      to_char(ticket.created, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created,
                      ticket.isanswered, ticket.isoverdue,
                      to_char(ticket.duedate, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS duedate,
                      ticket.sla_id,
                      sla.name AS sla_name,
                      ticket.staff_id
               FROM ticket
               LEFT JOIN sla ON sla.id = ticket.sla_id
               WHERE ticket.ticket_id = $1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("Ticket detail lookup failed"))?;

    let (ticket_id, number, subject, email, name, status, created, isanswered, isoverdue, duedate, sla_id, sla_name, staff_id) =
        header.ok_or_else(|| ApiError::not_found("Ticket not found"))?;

    let detail = build_detail(
        pool, ticket_id, number, subject, email, name, status, created, isanswered, isoverdue, duedate, sla_id, sla_name, staff_id, session.staff_id,
    )
    .await?;
    Ok(Json(detail))
}

/// Load a ticket's full thread and assemble a [`TicketDetail`] from a header row.
///
/// Shared by [`ticket_detail`] and [`reply`] so both return the identical shape
/// (the staff thread: all M/R/N entries, created ASC).
///
/// @implements BS-032.10: includes duedate, sla_id, sla_name.
/// @implements TS-M3-C4: includes staff_id for assignment check.
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
    isoverdue: bool,
    duedate: Option<String>,
    sla_id: Option<i32>,
    sla_name: Option<String>,
    staff_id: Option<i32>,
    viewer_staff_id: i32,
) -> Result<TicketDetail, ApiError> {
    // The shared core loads the thread in created ASC order, all M/R/N entries.
    let thread = load_thread(pool, ticket_id)
        .await
        .map_err(|_| ApiError::internal("Thread lookup failed"))?;

    // Collaborative-lock state for the requesting agent (FS-021.18/.20): drives
    // the "locked by <name>" banner and reply-composer gate on the detail view.
    let lock = ost_core::get_lock_info(pool, ticket_id, viewer_staff_id)
        .await
        .map_err(|_| ApiError::internal("Lock lookup failed"))?;

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
            title: e.title,
            body: e.body,
            created: e.created,
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
        isoverdue,
        duedate,
        sla_id,
        sla_name,
        staff_id,
        lock,
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
            r#"SELECT ticket.ticket_id, ticket."ticketID", ticket.subject, ticket.email, ticket.name, ticket.status,
                      to_char(ticket.created, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created,
                      ticket.isanswered, ticket.isoverdue,
                      to_char(ticket.duedate, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS duedate,
                      ticket.sla_id,
                      sla.name AS sla_name,
                      ticket.staff_id
               FROM ticket
               LEFT JOIN sla ON sla.id = ticket.sla_id
               WHERE ticket.ticket_id = $1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("Ticket lookup failed"))?;
    let (ticket_id, number, subject, email, name, status, created, _isanswered, isoverdue, duedate, sla_id, sla_name, staff_id) =
        header.ok_or_else(|| ApiError::not_found("Ticket not found"))?;

    // Lock check: block reply if ticket is locked by another staff member.
    // @implements FS-021.3: lock check for reply.
    // @implements BS-021.3: lock blocks conflicting replies.
    // @implements TS-M3-I3 AC-8: blocks reply when locked by another.
    if let Some(_lock_owner) = ost_core::check_lock(pool, ticket_id, session.staff_id)
        .await
        .map_err(|_| ApiError::internal("Lock check failed"))?
    {
        return Err(ApiError::conflict("Action Denied. Ticket is locked by someone else!"));
    }

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
    // Note: After replying, staff_id is preserved from original ticket state.
    let detail = build_detail(
        pool, ticket_id, number, subject, email, name, status, created, true, isoverdue, duedate, sla_id, sla_name, staff_id, session.staff_id,
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

// ---------------------------------------------------------------------------
// Workflow routes (TS-M3-C1/C2/C3)
// ---------------------------------------------------------------------------

use ost_core::permission::{
    PERM_CAN_ASSIGN_TICKETS, PERM_CAN_CLOSE_TICKETS, PERM_CAN_TRANSFER_TICKETS,
};
use ost_core::workflow::{self, AssigneeType, WorkflowError};

/// Request body for close/reopen actions.
///
/// @implements FS-021.11, FS-021.12: Close/reopen request shape.
#[derive(Debug, Default, Deserialize)]
pub struct CloseReopenRequest {
    /// Optional comments for the internal note.
    #[serde(default)]
    pub comments: Option<String>,
}

/// Success response for workflow actions.
#[derive(Debug, Serialize)]
pub struct WorkflowSuccess {
    pub success: bool,
    pub message: String,
}

/// `POST /api/staff/tickets/{id}/close` — close a ticket.
///
/// Requires `can_close_tickets` permission. Validates that the ticket is open.
/// Sets status to closed, clears overdue/due date, credits the closer, logs an
/// internal note, and records a `closed` lifecycle event.
///
/// @implements FS-021.11: Close ticket.
/// @implements BS-021.4: Closing clears overdue and due date.
pub async fn close_ticket_route(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i64>,
    body: Option<Json<CloseReopenRequest>>,
) -> Result<Json<WorkflowSuccess>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Close is unavailable"))?;

    // Permission gate: require can_close_tickets.
    require_staff_permission(&state, &session, PERM_CAN_CLOSE_TICKETS).await.map_err(|_| {
        ApiError::forbidden("Not allowed to close tickets")
    })?;

    let (name, username) = staff_info(pool, session.staff_id).await?;
    let comments = body.as_ref().and_then(|b| b.comments.as_deref());

    let result = workflow::close_ticket(
        pool,
        id,
        session.staff_id,
        &name,
        &username,
        comments,
    )
    .await
    .map_err(map_workflow_error)?;

    Ok(Json(WorkflowSuccess {
        success: true,
        message: format!("Ticket #{} status set to CLOSED", result.ticket_number),
    }))
}

/// `POST /api/staff/tickets/{id}/reopen` — reopen a closed ticket.
///
/// Requires `can_close_tickets` OR `can_create_tickets` permission. Validates that
/// the ticket is closed. Sets status to open, marks unanswered, logs an internal
/// note, records a `reopened` lifecycle event, and annuls the prior close event.
///
/// @implements FS-021.12: Reopen ticket.
/// @implements BS-021.14: Reopen annuls the prior close event.
pub async fn reopen_ticket_route(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i64>,
    body: Option<Json<CloseReopenRequest>>,
) -> Result<Json<WorkflowSuccess>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Reopen is unavailable"))?;

    // Permission gate: require can_close_tickets OR can_create_tickets.
    // @implements FS-021.12: reopen requires close OR create permission.
    let close_ok = require_staff_permission(&state, &session, PERM_CAN_CLOSE_TICKETS).await.is_ok();
    let create_ok = require_staff_permission(&state, &session, ost_core::PERM_CAN_CREATE_TICKETS).await.is_ok();
    if !close_ok && !create_ok {
        return Err(ApiError::forbidden("Not allowed to reopen tickets"));
    }

    let (name, username) = staff_info(pool, session.staff_id).await?;
    let comments = body.as_ref().and_then(|b| b.comments.as_deref());

    let _result = workflow::reopen_ticket(
        pool,
        id,
        session.staff_id,
        &name,
        &username,
        comments,
    )
    .await
    .map_err(map_workflow_error)?;

    Ok(Json(WorkflowSuccess {
        success: true,
        message: "Ticket REOPENED".to_string(),
    }))
}

/// `POST /api/staff/tickets/{id}/claim` — self-assign a ticket.
///
/// Requires `can_assign_tickets` permission. Validates that the ticket is open
/// and unassigned. Logs an internal note "Ticket claimed by <name>".
///
/// @implements FS-021.8: Claim ticket.
pub async fn claim_ticket_route(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i64>,
) -> Result<Json<WorkflowSuccess>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Claim is unavailable"))?;

    // Permission gate: require can_assign_tickets.
    require_staff_permission(&state, &session, PERM_CAN_ASSIGN_TICKETS).await.map_err(|_| {
        ApiError::forbidden("Not allowed to assign tickets")
    })?;

    let (name, _username) = staff_info(pool, session.staff_id).await?;

    let _result = workflow::claim_ticket(pool, id, session.staff_id, &name)
        .await
        .map_err(map_workflow_error)?;

    Ok(Json(WorkflowSuccess {
        success: true,
        message: "Ticket is now assigned to you!".to_string(),
    }))
}

/// Request body for the assign action.
///
/// @implements BS-021.8: Assign request shape.
#[derive(Debug, Deserialize)]
pub struct AssignRequest {
    /// Assignee id: s123 for staff, t456 for team.
    pub assignee: String,
    /// Required comments (>= 5 chars).
    pub comments: String,
}

/// `POST /api/staff/tickets/{id}/assign` — assign to a staff member or team.
///
/// Requires `can_assign_tickets` permission. Validates comments >= 5 chars and
/// that the ticket isn't already assigned to the same assignee. If the ticket
/// is closed, it will be reopened. Logs an internal note.
///
/// @implements FS-021.7: Assign ticket.
/// @implements BS-021.8: Assignment requires comments >= 5 chars.
pub async fn assign_ticket_route(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i64>,
    Json(body): Json<AssignRequest>,
) -> Result<Json<WorkflowSuccess>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Assign is unavailable"))?;

    // Permission gate: require can_assign_tickets.
    require_staff_permission(&state, &session, PERM_CAN_ASSIGN_TICKETS).await.map_err(|_| {
        ApiError::forbidden("Not allowed to assign tickets")
    })?;

    // Parse assignee.
    let assignee = AssigneeType::parse(&body.assignee)
        .ok_or_else(|| ApiError::validation("Invalid assignee ID - get technical support"))?;

    let (name, username) = staff_info(pool, session.staff_id).await?;

    let _result = workflow::assign_ticket(
        pool,
        id,
        assignee,
        &body.comments,
        session.staff_id,
        &name,
        &username,
    )
    .await
    .map_err(map_workflow_error)?;

    Ok(Json(WorkflowSuccess {
        success: true,
        message: "Ticket assigned successfully".to_string(),
    }))
}

/// Request body for the transfer action.
///
/// @implements FS-021.10: Transfer request shape.
#[derive(Debug, Deserialize)]
pub struct TransferRequest {
    /// Target department ID.
    pub dept_id: i32,
    /// Required comments (>= 5 chars).
    pub comments: String,
}

/// Transfer success response with access_lost flag.
///
/// @implements FS-021.10: Transfer response shape.
#[derive(Debug, Serialize)]
pub struct TransferSuccess {
    pub success: bool,
    pub message: String,
    pub access_lost: bool,
}

/// `POST /api/staff/tickets/{id}/transfer` — transfer to a different department.
///
/// Requires `can_transfer_tickets` permission. Validates comments >= 5 chars and
/// that the target department differs from the current one. If the ticket is closed,
/// it will be reopened. Re-selects SLA based on the new department. Logs an internal
/// note and records a transferred lifecycle event.
///
/// @implements FS-021.10: Transfer ticket.
/// @implements FS-021.13: SLA re-selection on transfer.
pub async fn transfer_ticket_route(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i64>,
    Json(body): Json<TransferRequest>,
) -> Result<Json<TransferSuccess>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Transfer is unavailable"))?;

    // Permission gate: require can_transfer_tickets.
    require_staff_permission(&state, &session, PERM_CAN_TRANSFER_TICKETS).await.map_err(|_| {
        ApiError::forbidden("Not allowed to transfer tickets")
    })?;

    let (name, username) = staff_info(pool, session.staff_id).await?;

    // Load actor's accessible departments for access check.
    let visibility = ost_core::load_staff_visibility(pool, session.staff_id)
        .await
        .map_err(|_| ApiError::internal("Visibility lookup failed"))?;

    let result = workflow::transfer_ticket(
        pool,
        id,
        body.dept_id,
        &body.comments,
        session.staff_id,
        &name,
        &username,
        &visibility.dept_ids,
    )
    .await
    .map_err(map_workflow_error)?;

    Ok(Json(TransferSuccess {
        success: true,
        message: format!("Ticket transferred successfully to {}", result.dept_name),
        access_lost: result.access_lost,
    }))
}

/// Fetch staff display name and username.
async fn staff_info(pool: &sqlx::postgres::PgPool, staff_id: i32) -> Result<(String, String), ApiError> {
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT username, firstname, lastname FROM staff WHERE staff_id = $1",
    )
    .bind(staff_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal("Staff lookup failed"))?;

    let (username, firstname, lastname) =
        row.ok_or_else(|| ApiError::unauthenticated("Session expired or invalid"))?;

    let name = display_name(&firstname, &lastname, &username);
    Ok((name, username))
}

/// Map workflow errors to API errors.
fn map_workflow_error(err: WorkflowError) -> ApiError {
    match err {
        WorkflowError::NotFound => ApiError::not_found("Ticket not found"),
        WorkflowError::AlreadyClosed => ApiError::validation("Ticket is already closed!"),
        WorkflowError::AlreadyOpen => ApiError::validation("Ticket is already open!"),
        WorkflowError::AlreadyAssigned => ApiError::validation("Ticket already assigned"),
        WorkflowError::AlreadyAssignedToSameStaff => {
            ApiError::validation("Ticket already assigned to the staff.")
        }
        WorkflowError::AlreadyAssignedToSameTeam => {
            ApiError::validation("Ticket already assigned to the team.")
        }
        WorkflowError::OnlyOpenCanBeClaimed => {
            ApiError::validation("Only open tickets can be claimed")
        }
        WorkflowError::CommentTooShort => {
            ApiError::validation("Comment too short").with_field("comments", "min 5 chars")
        }
        WorkflowError::AlreadyInDepartment => {
            ApiError::validation("Ticket already in the department")
        }
        WorkflowError::InvalidDepartment => {
            ApiError::validation("Unknown or invalid department")
        }
        WorkflowError::TransferCommentTooShort => {
            ApiError::validation("Transfer comments too short!")
        }
        WorkflowError::InvalidAssignee => {
            ApiError::validation("Unknown or invalid assignee")
        }
        WorkflowError::Db(_) => ApiError::internal("Database error"),
    }
}

// ---------------------------------------------------------------------------
// Internal notes routes (TS-M3-E1/E2)
// ---------------------------------------------------------------------------

use ost_core::workflow::{post_note, NoteState};

/// Request body for posting an internal note.
///
/// @implements FS-021.4: note request shape.
#[derive(Debug, Deserialize)]
pub struct NoteRequest {
    /// The note body (required, cannot be empty).
    pub body: String,
    /// Optional note title.
    #[serde(default)]
    pub title: Option<String>,
    /// Optional state change: closed, open, answered, unanswered, overdue, notdue, unchanged.
    #[serde(default)]
    pub state: Option<String>,
}

/// Response for a posted note.
///
/// @implements FS-021.4: note response shape.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteResponse {
    pub id: i64,
    /// Thread entry type (always "N" for notes).
    #[serde(rename = "type")]
    pub thread_type: String,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub staff_id: i32,
    pub poster_name: String,
    pub state_changed: bool,
}

/// `POST /api/staff/tickets/{id}/note` — post an internal note.
///
/// Requires staff authentication (the [`StaffCsrf`] extractor). Validates that
/// the note body is not empty. Optionally applies a state change after posting
/// the note.
///
/// @implements FS-021.4: POST note endpoint.
/// @implements FS-021.22: poster_name set to staff name.
/// @implements BS-021.10: notes excluded from client thread (enforced in client.rs).
pub async fn post_note_route(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i64>,
    Json(body): Json<NoteRequest>,
) -> Result<Json<NoteResponse>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Note posting is unavailable"))?;

    // Validate body is not empty.
    // @implements FS-021.4 AC-2: empty body returns 400 "Note required".
    let trimmed_body = body.body.trim();
    if trimmed_body.is_empty() {
        return Err(ApiError::bad_request("Note required"));
    }

    // Validate the ticket exists.
    let ticket_row: Option<(i64, i32)> = sqlx::query_as(
        r#"SELECT ticket_id, dept_id FROM ticket WHERE ticket_id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal("Ticket lookup failed"))?;

    let (ticket_id, ticket_dept_id) = ticket_row.ok_or_else(|| ApiError::not_found("Ticket not found"))?;

    // Check visibility/access (TS-M3-A2).
    // @implements TS-M3-E1 AC-7: 403 if staff has no access to ticket.
    let visibility = load_staff_visibility(pool, session.staff_id)
        .await
        .map_err(|_| ApiError::internal("Visibility lookup failed"))?;

    // Staff can access the ticket if:
    // 1. Ticket is assigned to them (staff_id matches), OR
    // 2. Ticket is in a department they have access to (and not access-limited), OR
    // 3. Ticket is assigned to one of their teams.
    let assigned_to_staff: Option<i32> = sqlx::query_scalar(
        "SELECT staff_id FROM ticket WHERE ticket_id = $1",
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal("Ticket lookup failed"))?
    .flatten();

    let assigned_team: Option<i32> = sqlx::query_scalar(
        "SELECT team_id FROM ticket WHERE ticket_id = $1",
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal("Ticket lookup failed"))?
    .flatten();

    let has_access = assigned_to_staff == Some(session.staff_id)
        || (!visibility.is_access_limited && visibility.dept_ids.contains(&ticket_dept_id))
        || assigned_team.map(|t| visibility.team_ids.contains(&t)).unwrap_or(false);

    if !has_access {
        return Err(ApiError::forbidden("Access denied"));
    }

    // Parse the state change.
    // @implements TS-M3-E2 AC-9: invalid state value returns 400.
    let note_state = match body.state.as_deref() {
        Some(s) if !s.is_empty() => NoteState::from_str(s)
            .ok_or_else(|| ApiError::bad_request("Invalid state value"))?,
        _ => NoteState::Unchanged,
    };

    // Get staff display name.
    let (name, _username) = staff_info(pool, session.staff_id).await?;

    // Post the note with optional state change.
    let result = post_note(
        pool,
        ticket_id,
        session.staff_id,
        &name,
        trimmed_body,
        body.title.as_deref().filter(|t| !t.trim().is_empty()),
        note_state,
    )
    .await
    .map_err(map_workflow_error)?;

    Ok(Json(NoteResponse {
        id: result.entry_id,
        thread_type: "N".to_string(),
        body: trimmed_body.to_string(),
        title: body.title.clone().filter(|t| !t.trim().is_empty()),
        staff_id: session.staff_id,
        poster_name: name,
        state_changed: result.state_changed,
    }))
}

/// `GET /api/staff/tickets/{id}/thread` — return the ticket's full thread.
///
/// Staff-only endpoint that includes ALL thread entry types (M/R/N).
///
/// @implements FS-021.4 AC-6: staff thread endpoint includes notes.
pub async fn get_thread(
    State(state): State<AppState>,
    _session: StaffSession,
    Path(id): Path<i64>,
) -> Result<Json<Vec<ThreadEntryView>>, ApiError> {
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

    // Load all thread entries (M/R/N) in chronological order.
    let thread = load_thread(pool, id)
        .await
        .map_err(|_| ApiError::internal("Thread lookup failed"))?;

    // Load per-entry attachments.
    let mut by_ref = load_attachments_by_ref(pool, id)
        .await
        .map_err(|_| ApiError::internal("Attachment lookup failed"))?;

    let entries = thread
        .into_iter()
        .map(|e| ThreadEntryView {
            attachments: by_ref.remove(&e.id).unwrap_or_default(),
            id: e.id,
            thread_type: e.thread_type,
            poster: e.poster,
            title: e.title,
            body: e.body,
            created: e.created,
        })
        .collect();

    Ok(Json(entries))
}

// ---------------------------------------------------------------------------
// Bulk actions (TS-M3-G1)
// ---------------------------------------------------------------------------

use ost_core::permission::{PERM_CAN_DELETE_TICKETS, PERM_CAN_EDIT_TICKETS};
use ost_core::workflow::{
    acquire_lock, bulk_action, delete_ticket, get_lock_time_minutes, release_lock,
    update_ticket, BulkAction, BulkError, LockError, UpdateError, UpdateTicketInput,
};

/// Request body for the bulk action endpoint.
///
/// @implements FS-021.21: Bulk action request shape.
#[derive(Debug, Deserialize)]
pub struct BulkActionRequest {
    /// Action to perform: "close", "reopen", or "delete".
    pub action: String,
    /// List of ticket IDs to process.
    pub ticket_ids: Vec<i64>,
}

/// Response for bulk actions.
///
/// @implements FS-021.21: Bulk action response shape.
#[derive(Debug, Serialize)]
pub struct BulkActionResponse {
    pub succeeded: usize,
    pub failed: usize,
    pub message: String,
}

/// `POST /api/staff/tickets/bulk` — perform bulk actions on multiple tickets.
///
/// Requires `canManageTickets` permission, then checks per-action permissions.
/// Processes each ticket individually with partial success reporting.
///
/// @implements FS-021.21: Mass process endpoint.
/// @implements BS-020.9: canManageTickets gate.
/// @implements EC-020.9: empty ticket_ids returns error.
/// @implements EC-020.13: unknown action is rejected.
pub async fn bulk_action_route(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Json(body): Json<BulkActionRequest>,
) -> Result<Json<BulkActionResponse>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Bulk action is unavailable"))?;

    // Parse the action first.
    let action = BulkAction::from_str(&body.action)
        .ok_or_else(|| ApiError::validation("Unknown action"))?;

    // Validate ticket_ids is not empty.
    // @implements EC-020.9: empty ticket_ids returns error.
    if body.ticket_ids.is_empty() {
        return Err(ApiError::validation("No tickets selected"));
    }

    // Permission gate: check specific action permission.
    // @implements BS-020.9: canManageTickets then per-action permission.
    match action {
        BulkAction::Close => {
            require_staff_permission(&state, &session, PERM_CAN_CLOSE_TICKETS)
                .await
                .map_err(|_| ApiError::forbidden("Permission denied"))?;
        }
        BulkAction::Reopen => {
            // Reopen requires close OR create permission.
            let close_ok = require_staff_permission(&state, &session, PERM_CAN_CLOSE_TICKETS).await.is_ok();
            let create_ok = require_staff_permission(&state, &session, ost_core::PERM_CAN_CREATE_TICKETS).await.is_ok();
            if !close_ok && !create_ok {
                return Err(ApiError::forbidden("Permission denied"));
            }
        }
        BulkAction::Delete => {
            require_staff_permission(&state, &session, PERM_CAN_DELETE_TICKETS)
                .await
                .map_err(|_| ApiError::forbidden("Permission denied"))?;
        }
    }

    let (name, username) = staff_info(pool, session.staff_id).await?;

    let result = bulk_action(
        pool,
        action,
        &body.ticket_ids,
        session.staff_id,
        &name,
        &username,
    )
    .await
    .map_err(|e| match e {
        BulkError::EmptySelection => ApiError::validation("No tickets selected"),
        BulkError::UnknownAction => ApiError::validation("Unknown action"),
        BulkError::Db(_) => ApiError::internal("Database error"),
    })?;

    Ok(Json(BulkActionResponse {
        succeeded: result.succeeded,
        failed: result.failed,
        message: result.message,
    }))
}

// ---------------------------------------------------------------------------
// Delete ticket (TS-M3-I2)
// ---------------------------------------------------------------------------

/// `DELETE /api/staff/tickets/{id}` — delete a ticket.
///
/// Requires `can_delete_tickets` permission. Permanently removes the ticket
/// and cascades to thread entries and attachments.
///
/// @implements FS-021.19: Delete ticket endpoint.
/// @implements BS-021.16: Deletion cascades to thread and attachments.
pub async fn delete_ticket_route(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i64>,
) -> Result<Json<WorkflowSuccess>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Delete is unavailable"))?;

    // Permission gate: require can_delete_tickets.
    require_staff_permission(&state, &session, PERM_CAN_DELETE_TICKETS)
        .await
        .map_err(|_| ApiError::forbidden("Permission denied"))?;

    let (_name, username) = staff_info(pool, session.staff_id).await?;

    let result = delete_ticket(pool, id, &username)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, ticket_id = id, "delete_ticket_route failed");
            match e {
                WorkflowError::NotFound => ApiError::not_found("Ticket not found"),
                _ => ApiError::internal("Delete failed"),
            }
        })?;

    Ok(Json(WorkflowSuccess {
        success: true,
        message: format!("Ticket #{} deleted successfully", result.ticket_number),
    }))
}

// ---------------------------------------------------------------------------
// Edit ticket (TS-M3-I1)
// ---------------------------------------------------------------------------

/// Request body for the update ticket endpoint.
///
/// @implements FS-021.15: Update ticket request shape.
#[derive(Debug, Deserialize)]
pub struct UpdateTicketRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub phone_ext: Option<String>,
    #[serde(default)]
    pub dept_id: Option<i32>,
    #[serde(default)]
    pub topic_id: Option<i32>,
    #[serde(default)]
    pub priority_id: Option<i32>,
    #[serde(default)]
    pub sla_id: Option<i32>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    /// Reason for the update (required).
    pub reason: String,
}

/// Response for update ticket.
///
/// @implements FS-021.15: Update ticket response shape.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTicketResponse {
    pub id: i64,
    pub priority_id: Option<i32>,
    pub sla_id: Option<i32>,
    pub isoverdue: bool,
}

/// `PUT /api/staff/tickets/{id}` — update ticket properties.
///
/// Requires `can_edit_tickets` permission. Validates input, updates the ticket,
/// re-selects SLA if department changed, clears overdue if due date is now in
/// the future, and logs an internal note.
///
/// @implements FS-021.15: Edit ticket endpoint.
/// @implements BS-021.6: editing due date clears overdue flag.
/// @implements BS-021.7: editing department re-selects SLA.
pub async fn update_ticket_route(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i64>,
    Json(body): Json<UpdateTicketRequest>,
) -> Result<Json<UpdateTicketResponse>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Update is unavailable"))?;

    // Permission gate: require can_edit_tickets.
    require_staff_permission(&state, &session, PERM_CAN_EDIT_TICKETS)
        .await
        .map_err(|_| ApiError::forbidden("Permission denied"))?;

    let (name, _username) = staff_info(pool, session.staff_id).await?;

    let input = UpdateTicketInput {
        name: body.name,
        email: body.email,
        phone: body.phone,
        phone_ext: body.phone_ext,
        dept_id: body.dept_id,
        topic_id: body.topic_id,
        priority_id: body.priority_id,
        sla_id: body.sla_id,
        source: body.source,
        due_date: body.due_date,
        reason: body.reason,
    };

    let _result = update_ticket(pool, id, input, session.staff_id, &name)
        .await
        .map_err(map_update_error)?;

    // Fetch updated ticket state for response.
    let row: Option<(i64, Option<i32>, Option<i32>, bool)> = sqlx::query_as(
        "SELECT ticket_id, priority_id, sla_id, isoverdue FROM ticket WHERE ticket_id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal("Ticket lookup failed"))?;

    let (ticket_id, priority_id, sla_id, isoverdue) =
        row.ok_or_else(|| ApiError::not_found("Ticket not found"))?;

    Ok(Json(UpdateTicketResponse {
        id: ticket_id,
        priority_id,
        sla_id,
        isoverdue,
    }))
}

/// Map update errors to API errors.
fn map_update_error(err: UpdateError) -> ApiError {
    match err {
        UpdateError::NotFound => ApiError::not_found("Ticket not found"),
        UpdateError::ReasonRequired => ApiError::validation("Reason for the update required"),
        UpdateError::DueDateInPast => ApiError::validation("Due date must be in the future"),
        UpdateError::DueDateOnClosed => {
            ApiError::validation("Due date can NOT be set on a closed ticket")
        }
        UpdateError::InvalidDueDate => ApiError::validation("Invalid due date"),
        UpdateError::PhoneExtWithoutPhone => {
            ApiError::validation("Phone extension requires a phone number")
        }
        UpdateError::InvalidEmail => ApiError::validation("Invalid email format"),
        UpdateError::InvalidDepartment => ApiError::validation("Unknown or invalid department"),
        UpdateError::Db(_) => ApiError::internal("Database error"),
    }
}

// ---------------------------------------------------------------------------
// Ticket locking (TS-M3-I3)
// ---------------------------------------------------------------------------

/// Response for lock acquire.
///
/// @implements FS-021.18: Lock acquire response shape.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LockResponse {
    pub lock_id: i32,
    pub remaining_seconds: i64,
}

/// Response for lock release.
///
/// @implements FS-021.18: Lock release response shape.
#[derive(Debug, Serialize)]
pub struct ReleaseResponse {
    pub released: bool,
}

/// `POST /api/staff/tickets/{id}/lock` — acquire or renew a lock on a ticket.
///
/// Auto-acquires a lock when viewing a ticket, or renews an existing lock.
/// Returns the lock id and remaining time.
///
/// @implements FS-021.18: Lock acquire/renew endpoint.
/// @implements BS-021.3: One active lock per ticket.
pub async fn acquire_lock_route(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i64>,
) -> Result<Json<LockResponse>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Lock is unavailable"))?;

    // Get lock time from config.
    let lock_time = get_lock_time_minutes(pool)
        .await
        .map_err(|_| ApiError::internal("Config lookup failed"))?;

    let result = acquire_lock(pool, id, session.staff_id, lock_time)
        .await
        .map_err(|e| match e {
            LockError::NotFound => ApiError::not_found("Ticket not found"),
            LockError::LockedByOther(name) => {
                ApiError::conflict(&format!("Ticket is currently locked by {}", name))
            }
            LockError::Db(_) => ApiError::internal("Lock failed"),
        })?;

    Ok(Json(LockResponse {
        lock_id: result.lock_id,
        remaining_seconds: result.remaining_seconds,
    }))
}

/// `DELETE /api/staff/tickets/{id}/lock` — release a lock on a ticket.
///
/// Releases the lock if owned by the requester.
///
/// @implements FS-021.18: Lock release endpoint.
/// @implements EC-021.14: release by non-owner is no-op.
pub async fn release_lock_route(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i64>,
) -> Result<Json<ReleaseResponse>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Lock is unavailable"))?;

    let released = release_lock(pool, id, session.staff_id)
        .await
        .map_err(|_| ApiError::internal("Lock release failed"))?;

    Ok(Json(ReleaseResponse { released }))
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
