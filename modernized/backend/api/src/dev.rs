//! Env-gated developer endpoints (TS-M1-A4b, TS-M3-A1, TS-M3-A2).
//!
//! @implements FS-040: `GET /api/dev/mailbox` surfaces the stub mailer's
//!   recorded sends so qa-criterion-tester can verify reply notifications
//!   without a real SMTP transport. **Disabled in production** (returns 404 so
//!   the route's existence is not even advertised).
//! @implements TS-M3-A1: `POST /api/dev/seed-tickets` creates test tickets with
//!   configurable status/flags for testing queue filtering.
//! @implements TS-M3-A2: `POST /api/dev/seed-staff` creates test staff accounts
//!   with custom visibility settings (show_assigned_only, dept_ids).

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use http::StatusCode;
use serde::{Deserialize, Serialize};

use ost_core::ticket::{
    append_thread_entry, create_ticket, NewThreadEntry, NewTicket, NewTicketInput, ThreadType,
};
use ost_core::ApiError;

use crate::state::AppState;

/// `GET /api/dev/mailbox` — return the recorded stub-mailer sends as JSON.
///
/// In production (`APP_ENV=production`) the dev endpoints are disabled and this
/// returns a 404 error envelope, so the endpoint is invisible there.
///
/// @implements FS-040: dev mailbox (dev-only).
pub async fn mailbox(State(state): State<AppState>) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let recorded = state.mailer.recorder.recorded();
    (StatusCode::OK, Json(recorded)).into_response()
}

/// Optional shaping for the dev seed-ticket endpoint.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedTicketRequest {
    /// Also append a staff response (`R`) to the seeded ticket.
    #[serde(default)]
    pub with_reply: bool,
    /// Also append an internal note (`N`) to the seeded ticket.
    #[serde(default)]
    pub with_note: bool,
}

/// `POST /api/dev/seed-ticket` — create an isolated ticket for QA, returning the
/// data the client portal needs to log in: `{ ticketNumber, email }`.
///
/// Disabled in production (returns a 404 envelope, like the dev mailbox) so the
/// endpoint is invisible there. Optionally seeds a staff reply (`R`) and an
/// internal note (`N`) so the M/R-only client-thread security AC has an `N` to
/// prove is excluded. Each call uses a fresh unique email so seeded tickets do
/// not collide on the composite (ticketID, email) identity.
///
/// @implements FS-010 (test infra): isolated ticket fixture for the client E2E.
pub async fn seed_ticket(
    State(state): State<AppState>,
    body: Option<Json<SeedTicketRequest>>,
) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let Json(req) = body.unwrap_or_default();

    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Seeding is unavailable").into_response(),
    };

    // A fresh unique email per seed keeps the (ticketID, email) identity distinct.
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let email = format!("qa+{nonce}@example.com");

    let new_ticket = match NewTicket::validated(NewTicketInput {
        email: email.clone(),
        name: "QA Seed".to_string(),
        subject: "Seeded ticket".to_string(),
        body: "Seeded ticket message.".to_string(),
        source: Some("Web".to_string()),
        dept_id: None,
    }) {
        Ok(t) => t,
        Err(_) => return ApiError::internal("Seed input invalid").into_response(),
    };

    let ticket = match create_ticket(pool, &new_ticket).await {
        Ok(t) => t,
        Err(_) => return ApiError::internal("Could not seed ticket").into_response(),
    };

    if req.with_reply {
        let r = NewThreadEntry::response("QA Agent", None, "Seeded staff reply.");
        if let Err(e) = append_thread_entry(pool, ticket.ticket_id, &r).await {
            tracing::warn!(error = %e, "seed_ticket: reply append failed");
        }
    }
    if req.with_note {
        let n = NewThreadEntry {
            thread_type: ThreadType::Note,
            poster: "QA Agent".to_string(),
            staff_id: None,
            body: "Seeded internal note (must never reach the client).".to_string(),
        };
        if let Err(e) = append_thread_entry(pool, ticket.ticket_id, &n).await {
            tracing::warn!(error = %e, "seed_ticket: note append failed");
        }
    }

    let resp = Json(serde_json::json!({
        "ticketNumber": ticket.ticket_number,
        "email": email,
    }));
    (StatusCode::CREATED, resp).into_response()
}

// --- TS-M3-A1: seed-tickets endpoint for queue testing -----------------------

/// A single ticket spec for the bulk seed endpoint.
///
/// @implements TS-M3-A1: seed tickets with configurable flags.
/// @implements TS-M3-A2: adds dept_id and team_id for visibility testing.
/// @implements TS-M3-A3: adds closed_by_staff_id for Closed By column testing.
/// @implements TS-M3-C3: adds duedate for close/overdue testing.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedTicketSpec {
    /// Ticket status: "open" (default) or "closed".
    #[serde(default = "default_status")]
    pub status: String,
    /// Whether the ticket is answered (default false).
    #[serde(default)]
    pub isanswered: bool,
    /// Whether the ticket is overdue (default false).
    #[serde(default)]
    pub isoverdue: bool,
    /// Staff ID to assign the ticket to (null = unassigned).
    #[serde(default)]
    pub staff_id: Option<i32>,
    /// Department ID for the ticket (null = use default dept).
    /// @implements TS-M3-A2: dept_id for visibility testing.
    #[serde(default)]
    pub dept_id: Option<i32>,
    /// Team ID to assign the ticket to (null = no team assignment).
    /// @implements TS-M3-A2: team_id for visibility testing.
    #[serde(default)]
    pub team_id: Option<i32>,
    /// Due date for the ticket (ISO 8601 format).
    /// @implements TS-M3-C3: duedate for close/overdue testing.
    #[serde(default)]
    pub duedate: Option<String>,
    /// Staff ID who closed the ticket (for Closed By column testing).
    /// @implements TS-M3-A3: closed_by_staff_id for rightmost column.
    #[serde(default)]
    pub closed_by_staff_id: Option<i32>,
}

fn default_status() -> String {
    "open".to_string()
}

/// Request body for bulk ticket seeding.
///
/// @implements TS-M3-A1: seed multiple tickets with configurable flags.
#[derive(Debug, Deserialize)]
pub struct SeedTicketsRequest {
    /// List of ticket specifications to create.
    pub tickets: Vec<SeedTicketSpec>,
}

/// Response for bulk ticket seeding.
///
/// @implements TS-M3-A1: returns created ticket IDs.
#[derive(Debug, Serialize)]
pub struct SeedTicketsResponse {
    /// List of created ticket IDs.
    pub ticket_ids: Vec<i64>,
}

/// `POST /api/dev/seed-tickets` — create multiple tickets with configurable flags.
///
/// Creates tickets with specific status, isanswered, isoverdue, staff_id, dept_id,
/// and team_id values for testing queue filtering and visibility. Disabled in
/// production (returns 404).
///
/// @implements TS-M3-A1: seed tickets with configurable flags for queue testing.
/// @implements TS-M3-A2: adds dept_id and team_id for visibility testing.
pub async fn seed_tickets(
    State(state): State<AppState>,
    body: Json<SeedTicketsRequest>,
) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }

    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Seeding is unavailable").into_response(),
    };

    let mut ticket_ids = Vec::new();

    for (idx, spec) in body.tickets.iter().enumerate() {
        // Unique email per ticket
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let email = format!("qa+seed{}_{nonce}@example.com", idx);

        let new_ticket = match NewTicket::validated(NewTicketInput {
            email: email.clone(),
            name: "QA Seed".to_string(),
            subject: format!("Seeded ticket {}", idx + 1),
            body: "Seeded ticket message.".to_string(),
            source: Some("Web".to_string()),
            dept_id: spec.dept_id,
        }) {
            Ok(t) => t,
            Err(_) => return ApiError::internal("Seed input invalid").into_response(),
        };

        let ticket = match create_ticket(pool, &new_ticket).await {
            Ok(t) => t,
            Err(_) => return ApiError::internal("Could not seed ticket").into_response(),
        };

        // Update the ticket with the specified flags
        let status = match spec.status.as_str() {
            "closed" => "closed",
            _ => "open",
        };

        // Update with all configurable fields including dept_id, team_id, and duedate.
        // @implements TS-M3-A2: set dept_id and team_id for visibility testing.
        // @implements TS-M3-C3: set duedate for close/overdue testing.
        // duedate is parsed server-side using PostgreSQL's timestamp parsing.
        // @implements TS-M3-A3: closed_by_staff_id for rightmost column testing.
        let duedate_sql = spec.duedate.as_deref().filter(|s| !s.is_empty());
        if let Err(e) = sqlx::query(
            "UPDATE ticket SET status = $1, isanswered = $2, isoverdue = $3, staff_id = $4, \
             dept_id = COALESCE($5, dept_id), team_id = COALESCE($6, 0), \
             duedate = CASE WHEN $7 IS NOT NULL THEN $7::timestamptz ELSE NULL END, \
             closed_by_staff_id = $8 \
             WHERE ticket_id = $9",
        )
        .bind(status)
        .bind(spec.isanswered)
        .bind(spec.isoverdue)
        .bind(spec.staff_id)
        .bind(spec.dept_id)
        .bind(spec.team_id.unwrap_or(0))
        .bind(duedate_sql)
        .bind(spec.closed_by_staff_id)
        .bind(ticket.ticket_id)
        .execute(pool)
        .await
        {
            tracing::warn!(error = %e, "seed_tickets: update failed");
            return ApiError::internal("Could not update seeded ticket").into_response();
        }

        ticket_ids.push(ticket.ticket_id);
    }

    let resp = Json(SeedTicketsResponse { ticket_ids });
    (StatusCode::CREATED, resp).into_response()
}

// --- TS-M3-A2: seed-staff endpoint for visibility testing --------------------

/// Request body for creating a test staff account.
///
/// @implements TS-M3-A2: seed staff with custom visibility settings.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedStaffRequest {
    /// Username for the new staff account.
    pub username: String,
    /// Password for the new staff account.
    pub password: String,
    /// Whether the staff member is access-limited (show_assigned_only).
    /// When true, they see only tickets assigned to them.
    #[serde(default)]
    pub show_assigned_only: bool,
    /// Department IDs this staff member's group should have access to.
    /// If empty, the staff will have no department access.
    #[serde(default)]
    pub dept_ids: Vec<i32>,
    /// Team IDs this staff member should belong to.
    #[serde(default)]
    pub team_ids: Vec<i32>,
    /// Reuse an existing group instead of minting a fresh one (TS-M4-B).
    #[serde(default)]
    pub group_id: Option<i32>,
    /// Home department id override (defaults to the first department).
    #[serde(default)]
    pub dept_id: Option<i32>,
    /// Directory-listing flag (TS-M4-B5 AC-5). Defaults to visible.
    #[serde(default = "default_true")]
    pub isvisible: bool,
    /// Administrator flag (TS-M4-B). Defaults to non-admin.
    #[serde(default)]
    pub isadmin: bool,
    /// Vacation flag (TS-M4-B). Defaults off.
    #[serde(default)]
    pub onvacation: bool,
    /// Optional first/last name (drives display + directory name matching).
    #[serde(default)]
    pub firstname: Option<String>,
    #[serde(default)]
    pub lastname: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Response for seed-staff endpoint.
///
/// @implements TS-M3-A2: returns created staff ID.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedStaffResponse {
    /// The created staff member's ID.
    pub staff_id: i32,
    /// The created group's ID (unique per test staff).
    pub group_id: i32,
}

/// `POST /api/dev/seed-staff` — create a test staff account with custom visibility.
///
/// Creates a staff account with specified username, password, show_assigned_only
/// flag, and department/team memberships. Each call creates a unique group for
/// the staff member with the specified department access.
///
/// Disabled in production (returns 404).
///
/// @implements TS-M3-A2: seed staff for visibility testing.
pub async fn seed_staff(
    State(state): State<AppState>,
    body: Json<SeedStaffRequest>,
) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }

    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Seeding is unavailable").into_response(),
    };

    // Hash the password.
    let passwd = match ost_core::hash_password(&body.password) {
        Ok(h) => h,
        Err(_) => return ApiError::internal("Password hashing failed").into_response(),
    };

    // Generate a unique group name for this test staff.
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let group_name = format!("Test Group {}", nonce);

    // Get the staff's home dept (required FK): explicit override, else first dept.
    let default_dept_id: i32 = match body.dept_id {
        Some(id) => id,
        None => match sqlx::query_scalar("SELECT dept_id FROM department ORDER BY dept_id LIMIT 1")
            .fetch_one(pool)
            .await
        {
            Ok(id) => id,
            Err(_) => return ApiError::internal("No department exists").into_response(),
        },
    };

    // Reuse an existing group if requested (TS-M4-B), else mint a fresh one.
    let group_id: i32 = match body.group_id {
        Some(id) => id,
        None => match sqlx::query_scalar(
            "INSERT INTO groups (group_name, group_enabled, can_create_tickets, can_post_reply)
             VALUES ($1, true, true, true)
             RETURNING group_id",
        )
        .bind(&group_name)
        .fetch_one(pool)
        .await
        {
            Ok(id) => id,
            Err(e) => {
                tracing::warn!(error = %e, "seed_staff: group creation failed");
                return ApiError::internal("Could not create group").into_response();
            }
        },
    };

    // Grant the group access to the specified departments.
    for &dept_id in &body.dept_ids {
        if let Err(e) = sqlx::query(
            "INSERT INTO group_dept_access (group_id, dept_id) VALUES ($1, $2)
             ON CONFLICT (group_id, dept_id) DO NOTHING",
        )
        .bind(group_id)
        .bind(dept_id)
        .execute(pool)
        .await
        {
            tracing::warn!(error = %e, "seed_staff: group_dept_access failed");
            return ApiError::internal("Could not grant department access").into_response();
        }
    }

    // Create the staff account.
    let staff_id: i32 = match sqlx::query_scalar(
        "INSERT INTO staff (group_id, dept_id, username, firstname, lastname, passwd, \
                            show_assigned_only, isactive, isvisible, isadmin, onvacation)
         VALUES ($1, $2, $3, $4, $5, $6, $7, true, $8, $9, $10)
         ON CONFLICT (username) DO UPDATE
           SET group_id = EXCLUDED.group_id,
               dept_id = EXCLUDED.dept_id,
               firstname = EXCLUDED.firstname,
               lastname = EXCLUDED.lastname,
               passwd = EXCLUDED.passwd,
               show_assigned_only = EXCLUDED.show_assigned_only,
               isvisible = EXCLUDED.isvisible,
               isadmin = EXCLUDED.isadmin,
               onvacation = EXCLUDED.onvacation,
               updated = now()
         RETURNING staff_id",
    )
    .bind(group_id)
    .bind(default_dept_id)
    .bind(&body.username)
    .bind(body.firstname.as_deref().unwrap_or(""))
    .bind(body.lastname.as_deref().unwrap_or(""))
    .bind(&passwd)
    .bind(body.show_assigned_only)
    .bind(body.isvisible)
    .bind(body.isadmin)
    .bind(body.onvacation)
    .fetch_one(pool)
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!(error = %e, "seed_staff: staff creation failed");
            return ApiError::internal("Could not create staff").into_response();
        }
    };

    // Add the staff to the specified teams.
    for &team_id in &body.team_ids {
        if let Err(e) = sqlx::query(
            "INSERT INTO team_member (team_id, staff_id) VALUES ($1, $2)
             ON CONFLICT (team_id, staff_id) DO NOTHING",
        )
        .bind(team_id)
        .bind(staff_id)
        .execute(pool)
        .await
        {
            tracing::warn!(error = %e, "seed_staff: team_member failed");
            // Non-fatal, continue.
        }
    }

    let resp = Json(SeedStaffResponse { staff_id, group_id });
    (StatusCode::CREATED, resp).into_response()
}

// --- TS-M3-C: dev endpoints for workflow testing ------------------------------

/// Request body for setting group permissions.
///
/// @implements TS-M3-C1/C2/C3: toggle group permissions for testing.
/// @implements TS-M3-G1/I1/I2: add delete and edit permissions.
/// @implements BS-021.1: add can_manage_tickets for bulk actions.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetGroupPermRequest {
    pub group_id: i32,
    #[serde(default)]
    pub can_close_tickets: Option<bool>,
    #[serde(default)]
    pub can_assign_tickets: Option<bool>,
    #[serde(default)]
    pub can_transfer_tickets: Option<bool>,
    #[serde(default)]
    pub can_create_tickets: Option<bool>,
    #[serde(default)]
    pub can_delete_tickets: Option<bool>,
    #[serde(default)]
    pub can_edit_tickets: Option<bool>,
    #[serde(default)]
    pub can_manage_tickets: Option<bool>,
    /// TS-M4-H1: the premade-manager delegated flag (canned-response CRUD gate).
    /// Accepts both the camelCase struct convention and the snake_case AC form.
    #[serde(default, alias = "can_manage_premade")]
    pub can_manage_premade: Option<bool>,
    /// TS-M4-H1 / EPIC-M4-E: the FAQ-manager delegated flag.
    #[serde(default, alias = "can_manage_faq")]
    pub can_manage_faq: Option<bool>,
}

/// `POST /api/dev/set-group-perm` — toggle group permission flags for testing.
///
/// Disabled in production (returns 404).
///
/// @implements TS-M3-C1/C2/C3: dev endpoint for permission testing.
/// @implements TS-M3-G1/I1/I2: adds delete and edit permissions.
/// @implements BS-021.1: adds can_manage_tickets for bulk actions.
pub async fn set_group_perm(
    State(state): State<AppState>,
    body: Json<SetGroupPermRequest>,
) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }

    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Permission update is unavailable").into_response(),
    };

    // Build dynamic UPDATE query based on which fields are present.
    let mut set_clauses: Vec<&str> = Vec::new();

    if body.can_close_tickets.is_some() {
        set_clauses.push("can_close_tickets = $2");
    }
    if body.can_assign_tickets.is_some() {
        set_clauses.push("can_assign_tickets = $3");
    }
    if body.can_transfer_tickets.is_some() {
        set_clauses.push("can_transfer_tickets = $4");
    }
    if body.can_create_tickets.is_some() {
        set_clauses.push("can_create_tickets = $5");
    }
    if body.can_delete_tickets.is_some() {
        set_clauses.push("can_delete_tickets = $6");
    }
    if body.can_edit_tickets.is_some() {
        set_clauses.push("can_edit_tickets = $7");
    }
    if body.can_manage_tickets.is_some() {
        set_clauses.push("can_manage_tickets = $8");
    }
    if body.can_manage_premade.is_some() {
        set_clauses.push("can_manage_premade = $9");
    }
    if body.can_manage_faq.is_some() {
        set_clauses.push("can_manage_faq = $10");
    }

    if set_clauses.is_empty() {
        return ApiError::validation("No permission flags provided").into_response();
    }

    // Build and execute the UPDATE statement.
    let query = format!(
        "UPDATE groups SET {} WHERE group_id = $1",
        set_clauses.join(", ")
    );

    if let Err(e) = sqlx::query(&query)
        .bind(body.group_id)
        .bind(body.can_close_tickets.unwrap_or(false))
        .bind(body.can_assign_tickets.unwrap_or(false))
        .bind(body.can_transfer_tickets.unwrap_or(false))
        .bind(body.can_create_tickets.unwrap_or(false))
        .bind(body.can_delete_tickets.unwrap_or(false))
        .bind(body.can_edit_tickets.unwrap_or(false))
        .bind(body.can_manage_tickets.unwrap_or(false))
        .bind(body.can_manage_premade.unwrap_or(false))
        .bind(body.can_manage_faq.unwrap_or(false))
        .execute(pool)
        .await
    {
        tracing::warn!(error = %e, "set_group_perm: update failed");
        return ApiError::internal("Could not update group permissions").into_response();
    }

    let resp = Json(serde_json::json!({ "success": true }));
    (StatusCode::OK, resp).into_response()
}

/// Request body for creating a test department.
///
/// @implements TS-M3-C2: seed departments for transfer testing.
#[derive(Debug, Deserialize)]
pub struct SeedDeptRequest {
    pub name: String,
    #[serde(default)]
    pub sla_id: Option<i32>,
}

/// Response for seed-dept endpoint.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedDeptResponse {
    pub dept_id: i32,
}

/// `POST /api/dev/seed-dept` — create a test department.
///
/// Disabled in production (returns 404).
///
/// @implements TS-M3-C2: dev endpoint for transfer testing.
pub async fn seed_dept(
    State(state): State<AppState>,
    body: Json<SeedDeptRequest>,
) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }

    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Seeding is unavailable").into_response(),
    };

    let dept_id: i32 = match sqlx::query_scalar(
        "INSERT INTO department (dept_name, sla_id) VALUES ($1, $2)
         ON CONFLICT (dept_name) DO UPDATE SET sla_id = EXCLUDED.sla_id
         RETURNING dept_id",
    )
    .bind(&body.name)
    .bind(body.sla_id)
    .fetch_one(pool)
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!(error = %e, "seed_dept: creation failed");
            return ApiError::internal("Could not create department").into_response();
        }
    };

    let resp = Json(SeedDeptResponse { dept_id });
    (StatusCode::CREATED, resp).into_response()
}

/// Request body for setting department SLA.
///
/// @implements TS-M3-C2: set department SLA for transfer testing.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDeptSlaRequest {
    pub dept_id: i32,
    pub sla_id: Option<i32>,
}

/// `POST /api/dev/set-dept-sla` — set a department's SLA.
///
/// Disabled in production (returns 404).
///
/// @implements TS-M3-C2 AC-8: set department SLA for testing.
pub async fn set_dept_sla(
    State(state): State<AppState>,
    body: Json<SetDeptSlaRequest>,
) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }

    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("SLA update is unavailable").into_response(),
    };

    if let Err(e) = sqlx::query("UPDATE department SET sla_id = $1 WHERE dept_id = $2")
        .bind(body.sla_id)
        .bind(body.dept_id)
        .execute(pool)
        .await
    {
        tracing::warn!(error = %e, "set_dept_sla: update failed");
        return ApiError::internal("Could not update department SLA").into_response();
    }

    let resp = Json(serde_json::json!({ "success": true }));
    (StatusCode::OK, resp).into_response()
}

/// Request body for creating an SLA plan.
///
/// @implements TS-M3-C2: seed SLA plans for testing.
#[derive(Debug, Deserialize)]
pub struct SeedSlaRequest {
    pub name: String,
    #[serde(default = "default_grace_hours")]
    pub hours: i32,
}

fn default_grace_hours() -> i32 {
    24
}

/// Response for seed-sla endpoint.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedSlaResponse {
    pub sla_id: i32,
}

/// `POST /api/dev/seed-sla` — create a test SLA plan.
///
/// Disabled in production (returns 404).
///
/// @implements TS-M3-C2 AC-8: seed SLA plans for testing.
pub async fn seed_sla(
    State(state): State<AppState>,
    body: Json<SeedSlaRequest>,
) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }

    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Seeding is unavailable").into_response(),
    };

    let sla_id: i32 = match sqlx::query_scalar(
        "INSERT INTO sla (name, grace_period) VALUES ($1, $2)
         ON CONFLICT (name) DO UPDATE SET grace_period = EXCLUDED.grace_period
         RETURNING id",
    )
    .bind(&body.name)
    .bind(body.hours)
    .fetch_one(pool)
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!(error = %e, "seed_sla: creation failed");
            return ApiError::internal("Could not create SLA").into_response();
        }
    };

    let resp = Json(SeedSlaResponse { sla_id });
    (StatusCode::CREATED, resp).into_response()
}

// --- TS-M3-E: dev endpoints for note/state testing ----------------------------

/// `POST /api/dev/mark-overdue/{id}` — mark a ticket as overdue for testing.
///
/// Disabled in production (returns 404).
///
/// @implements TS-M3-E2 AC-6: dev endpoint for state=notdue test.
pub async fn mark_overdue(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }

    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Update is unavailable").into_response(),
    };

    if let Err(e) = sqlx::query("UPDATE ticket SET isoverdue = true, updated = now() WHERE ticket_id = $1")
        .bind(id)
        .execute(pool)
        .await
    {
        tracing::warn!(error = %e, "mark_overdue: update failed");
        return ApiError::internal("Could not mark ticket overdue").into_response();
    }

    let resp = Json(serde_json::json!({ "success": true }));
    (StatusCode::OK, resp).into_response()
}

// --- TS-M3-F1/F2: dev endpoints for search testing ----------------------------

/// `POST /api/dev/reset-tickets` — delete all tickets for clean search testing state.
///
/// Disabled in production (returns 404).
///
/// @implements TS-M3-F1: dev endpoint for search testing (AC-1 setup).
pub async fn reset_tickets(State(state): State<AppState>) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }

    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Reset is unavailable").into_response(),
    };

    // Delete all tickets (cascade will remove thread entries, attachments, etc.).
    if let Err(e) = sqlx::query("DELETE FROM ticket").execute(pool).await {
        tracing::warn!(error = %e, "reset_tickets: delete failed");
        return ApiError::internal("Could not reset tickets").into_response();
    }

    let resp = Json(serde_json::json!({ "success": true, "message": "All tickets deleted" }));
    (StatusCode::OK, resp).into_response()
}

/// Request body for seeding a single search test ticket.
///
/// @implements TS-M3-F1/F2: seed tickets with configurable search-relevant fields.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedSearchTicketRequest {
    /// Ticket number (6-digit string). If provided, used as ticketID.
    #[serde(default, rename = "ticketId")]
    pub ticket_id: Option<String>,
    /// Requester email.
    #[serde(default)]
    pub email: Option<String>,
    /// Ticket subject.
    #[serde(default)]
    pub subject: Option<String>,
    /// Initial message body.
    #[serde(default)]
    pub body: Option<String>,
    /// Department name (will look up dept_id).
    #[serde(default)]
    pub dept: Option<String>,
    /// Help topic name (will look up topic_id).
    #[serde(default)]
    pub topic: Option<String>,
    /// Created date (ISO 8601 format, for date range testing).
    #[serde(default)]
    pub created: Option<String>,
    /// Ticket status: "open" (default) or "closed".
    #[serde(default)]
    pub status: Option<String>,
    /// Staff ID to assign the ticket to.
    #[serde(default)]
    pub staff_id: Option<i32>,
    /// Team ID to assign the ticket to.
    #[serde(default)]
    pub team_id: Option<i32>,
}

/// Response for single search ticket seeding.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedSearchTicketResponse {
    /// The created ticket's internal ID.
    pub ticket_id: i64,
    /// The ticket number.
    pub ticket_number: i64,
    /// The email used.
    pub email: String,
}

/// `POST /api/dev/seed-search-ticket` — create a single ticket for search testing.
///
/// Supports custom ticketId, email, subject, body, dept, topic, created date,
/// status, and assignment for comprehensive search testing scenarios.
///
/// Disabled in production (returns 404).
///
/// @implements TS-M3-F1: seed tickets for basic search testing.
/// @implements TS-M3-F2: seed tickets for advanced search testing.
pub async fn seed_search_ticket(
    State(state): State<AppState>,
    body: Json<SeedSearchTicketRequest>,
) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }

    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Seeding is unavailable").into_response(),
    };

    // Generate a unique email if not provided.
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let email = body.email.clone().unwrap_or_else(|| format!("qa+search{}@example.com", nonce));
    let subject = body.subject.clone().unwrap_or_else(|| "Search test ticket".to_string());
    let message_body = body.body.clone().unwrap_or_else(|| "Search test message body.".to_string());

    // Look up department ID if name provided.
    let dept_id: Option<i32> = if let Some(ref dept_name) = body.dept {
        sqlx::query_scalar("SELECT dept_id FROM department WHERE dept_name = $1")
            .bind(dept_name)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    // Look up topic ID if name provided.
    let topic_id: Option<i32> = if let Some(ref topic_name) = body.topic {
        sqlx::query_scalar("SELECT topic_id FROM help_topic WHERE topic = $1")
            .bind(topic_name)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    // Create the ticket using core function.
    let new_ticket = match NewTicket::validated(NewTicketInput {
        email: email.clone(),
        name: "Search Test User".to_string(),
        subject: subject.clone(),
        body: message_body.clone(),
        source: Some("Web".to_string()),
        dept_id,
    }) {
        Ok(t) => t,
        Err(_) => return ApiError::internal("Seed input invalid").into_response(),
    };

    let ticket = match create_ticket(pool, &new_ticket).await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(error = %e, "seed_search_ticket: create failed");
            return ApiError::internal("Could not seed ticket").into_response();
        }
    };

    // Update the ticket with custom fields.
    let mut updates: Vec<String> = Vec::new();

    // Custom ticket number.
    if let Some(ref custom_id) = body.ticket_id {
        if let Ok(num) = custom_id.parse::<i64>() {
            updates.push(format!(r#""ticketID" = {}"#, num));
        }
    }

    // Custom created date.
    if let Some(ref created) = body.created {
        if !created.is_empty() {
            updates.push(format!("created = '{}'::timestamptz", created.replace('\'', "''")));
        }
    }

    // Status.
    let status = body.status.as_deref().unwrap_or("open");
    updates.push(format!("status = '{}'", status));

    // Topic ID.
    if let Some(tid) = topic_id {
        updates.push(format!("topic_id = {}", tid));
    }

    // Staff assignment.
    if let Some(sid) = body.staff_id {
        updates.push(format!("staff_id = {}", sid));
    }

    // Team assignment.
    if let Some(tid) = body.team_id {
        updates.push(format!("team_id = {}", tid));
    }

    if !updates.is_empty() {
        let update_sql = format!(
            "UPDATE ticket SET {} WHERE ticket_id = $1",
            updates.join(", ")
        );
        if let Err(e) = sqlx::query(&update_sql)
            .bind(ticket.ticket_id)
            .execute(pool)
            .await
        {
            tracing::warn!(error = %e, "seed_search_ticket: update failed");
            // Non-fatal, continue with partial ticket.
        }
    }

    // Re-fetch the ticket number (in case we changed it).
    let ticket_number: i64 = sqlx::query_scalar(r#"SELECT "ticketID" FROM ticket WHERE ticket_id = $1"#)
        .bind(ticket.ticket_id)
        .fetch_one(pool)
        .await
        .unwrap_or(ticket.ticket_number);

    let resp = Json(SeedSearchTicketResponse {
        ticket_id: ticket.ticket_id,
        ticket_number,
        email,
    });
    (StatusCode::CREATED, resp).into_response()
}

/// `POST /api/dev/seed-topic` — create a help topic for search testing.
///
/// Disabled in production (returns 404).
///
/// @implements TS-M3-F2 AC-7: seed help topics for topic filter testing.
pub async fn seed_topic(
    State(state): State<AppState>,
    body: Json<SeedTopicRequest>,
) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }

    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Seeding is unavailable").into_response(),
    };

    let topic_id: i32 = match sqlx::query_scalar(
        "INSERT INTO help_topic (topic, isactive) VALUES ($1, true)
         ON CONFLICT (topic) DO UPDATE SET isactive = true
         RETURNING topic_id",
    )
    .bind(&body.name)
    .fetch_one(pool)
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!(error = %e, "seed_topic: creation failed");
            return ApiError::internal("Could not create help topic").into_response();
        }
    };

    let resp = Json(SeedTopicResponse { topic_id });
    (StatusCode::CREATED, resp).into_response()
}

/// Request for creating a help topic.
#[derive(Debug, Deserialize)]
pub struct SeedTopicRequest {
    pub name: String,
}

/// Response for seed-topic endpoint.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedTopicResponse {
    pub topic_id: i32,
}

// --- TS-M3-I3: dev endpoints for lock testing ---------------------------------

/// Request body for setting config values.
///
/// @implements TS-M3-I3: seed config for lock time testing.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedConfigRequest {
    /// Config key-value pairs to set.
    #[serde(flatten)]
    pub config: std::collections::HashMap<String, serde_json::Value>,
}

/// `POST /api/dev/seed-config` — set config values for testing.
///
/// Disabled in production (returns 404).
///
/// @implements TS-M3-I3 AC-1: set ticket_lock_time for testing.
pub async fn seed_config(
    State(state): State<AppState>,
    body: Json<SeedConfigRequest>,
) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }

    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Seeding is unavailable").into_response(),
    };

    for (key, value) in &body.config {
        let value_str = match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => if *b { "1" } else { "0" }.to_string(),
            _ => value.to_string(),
        };

        if let Err(e) = sqlx::query(
            "INSERT INTO config (key, value) VALUES ($1, $2)
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated = now()",
        )
        .bind(key)
        .bind(&value_str)
        .execute(pool)
        .await
        {
            tracing::warn!(error = %e, key = key, "seed_config: insert failed");
            return ApiError::internal("Could not set config value").into_response();
        }
    }

    let resp = Json(serde_json::json!({ "success": true }));
    (StatusCode::OK, resp).into_response()
}

// --- TS-M4-PREP: dev endpoints for M4 admin epics -----------------------------

/// Request body for seeding a syslog row.
///
/// @implements TS-M4-PREP (test infra): syslog has no writer yet; the M4-G log
///   viewer needs generated rows to display.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedLogRequest {
    /// Log type: one of `Error` | `Warning` | `Debug` (defaults to `Error`).
    #[serde(default = "default_log_type", rename = "type")]
    pub log_type: String,
    /// Log title / heading.
    pub title: String,
    /// Log body / detail.
    #[serde(default)]
    pub log: String,
    /// Optional `created` timestamp (ISO 8601). When omitted the row is stamped
    /// `now()`; backdated rows exercise the grace-period purge sweep (TS-M4-G1).
    #[serde(default)]
    pub created: Option<String>,
}

fn default_log_type() -> String {
    "Error".to_string()
}

/// `POST /api/dev/seed-log` — insert a syslog row for the M4-G log viewer E2E.
///
/// Disabled in production (returns 404).
///
/// Example request: `{"type":"Warning","title":"Cron skipped","log":"no due tickets"}`
/// Example response: `{"id":42}`.
///
/// @implements FS-033.1 (test infra): seed syslog rows (DEVIATION M4-D2).
pub async fn seed_log(State(state): State<AppState>, body: Json<SeedLogRequest>) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Seeding is unavailable").into_response(),
    };

    // Guard the enum at the app layer so a bad type gives a 422, not a 500.
    if !["Error", "Warning", "Debug"].contains(&body.log_type.as_str()) {
        return ApiError::validation("type must be one of Error|Warning|Debug").into_response();
    }

    // Optional backdated `created` (else now()): a NULL bind falls back to now()
    // via COALESCE, so the over-age purge ACs can seed a dated row.
    let created = body.created.as_deref().filter(|s| !s.trim().is_empty());
    let id: i64 = match sqlx::query_scalar(
        "INSERT INTO syslog (log_type, title, log, ip_address, created)
         VALUES ($1, $2, $3, '127.0.0.1', COALESCE($4::timestamptz, now()))
         RETURNING id",
    )
    .bind(&body.log_type)
    .bind(&body.title)
    .bind(&body.log)
    .bind(created)
    .fetch_one(pool)
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!(error = %e, "seed_log: insert failed");
            return ApiError::internal("Could not seed log").into_response();
        }
    };

    (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response()
}

/// `POST /api/dev/purge-logs` — run the real grace-period purge sweep
/// (TS-M4-G2): deletes only rows older than `log_graceperiod` months, or a no-op
/// when that key is unset/zero/non-numeric. This REPLACES the earlier
/// truncate-all behaviour (the cron trigger for the same sweep lands in M6).
///
/// Disabled in production (returns 404).
///
/// @implements BS-033.6: grace-period purge sweep (over-age rows only).
pub async fn purge_logs(State(state): State<AppState>) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Purge is unavailable").into_response(),
    };
    match ost_core::purge_logs(pool).await {
        Ok(deleted) => (
            StatusCode::OK,
            Json(serde_json::json!({ "success": true, "deleted": deleted })),
        )
            .into_response(),
        Err(e) => {
            tracing::warn!(error = %e, "purge_logs: sweep failed");
            ApiError::internal("Could not purge logs").into_response()
        }
    }
}

/// Request body for backdating a staff password (forced-change testing).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgePasswordRequest {
    /// The staff account to age.
    pub staff_id: i32,
    /// How many days to backdate `passwdreset` by.
    #[serde(default)]
    pub days: i32,
    /// Whether to also set `change_passwd = true`. Defaults to `true` (the pre-M4
    /// behaviour). Pass `false` to backdate `passwdreset` ONLY, so the login-time
    /// aging computation (FS-031.12) can be isolated (TS-M4-B5 AC-6).
    #[serde(default = "default_true")]
    pub force_change: bool,
}

/// `POST /api/dev/age-password` — backdate a staff `passwdreset` timestamp,
/// optionally setting `change_passwd`, so the forced-password-change flow can be
/// E2E'd without waiting for the real aging window.
///
/// Disabled in production (returns 404).
///
/// Example: `{"staffId":3,"days":120}` sets forced-change; `{"staffId":3,
/// "days":120,"forceChange":false}` backdates only (isolates the login-time
/// computation) → response `{"success":true}`.
///
/// @implements FS-031.12 (test infra): password-aging / forced-change test state.
pub async fn age_password(State(state): State<AppState>, body: Json<AgePasswordRequest>) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Update is unavailable").into_response(),
    };
    // Backdate passwdreset by `days`; set change_passwd only when requested.
    let interval = format!("{} days", body.days.max(0));
    if let Err(e) = sqlx::query(
        "UPDATE staff SET passwdreset = now() - $1::interval, \
                change_passwd = CASE WHEN $2 THEN true ELSE change_passwd END, updated = now()
         WHERE staff_id = $3",
    )
    .bind(&interval)
    .bind(body.force_change)
    .bind(body.staff_id)
    .execute(pool)
    .await
    {
        tracing::warn!(error = %e, "age_password: update failed");
        return ApiError::internal("Could not age password").into_response();
    }
    (StatusCode::OK, Json(serde_json::json!({ "success": true }))).into_response()
}

/// `POST /api/dev/reset-groups` — delete non-seed permission groups so a group
/// list / mass-action E2E starts clean (TS-M4-B3). Keeps the seed groups
/// (`M1 Agents`, `Administrators`) and skips any group that still has members
/// (BS-031-022 — a group with members cannot be deleted).
///
/// Disabled in production (returns 404).
///
/// @implements FS-031.7 (test infra): group-roster reset for the group-list E2E.
pub async fn reset_groups(State(state): State<AppState>) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Reset is unavailable").into_response(),
    };

    const KEEP: [&str; 2] = ["M1 Agents", "Administrators"];

    // Delete groups that are NOT seed groups AND have zero members. group_dept_access
    // is ON DELETE CASCADE, so their access rows go with them.
    let deleted: u64 = match sqlx::query(
        "DELETE FROM groups g \
         WHERE g.group_name <> ALL($1) \
           AND NOT EXISTS (SELECT 1 FROM staff s WHERE s.group_id = g.group_id)",
    )
    .bind(&KEEP[..])
    .execute(pool)
    .await
    {
        Ok(r) => r.rows_affected(),
        Err(e) => {
            tracing::warn!(error = %e, "reset_groups: delete failed");
            return ApiError::internal("Could not reset groups").into_response();
        }
    };

    (StatusCode::OK, Json(serde_json::json!({ "success": true, "deleted": deleted }))).into_response()
}

/// Request body for bulk-seeding staff (pagination ACs).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedStaffBulkRequest {
    /// How many staff accounts to create.
    pub count: i32,
    /// Username prefix; each account is `<prefix><n>` (defaults to `bulkstaff`).
    #[serde(default)]
    pub prefix: Option<String>,
    /// Reuse an existing group (defaults to a fresh shared group).
    #[serde(default)]
    pub group_id: Option<i32>,
    /// Home department (defaults to the first department).
    #[serde(default)]
    pub dept_id: Option<i32>,
    /// Directory-listing flag for the seeded rows (defaults visible).
    #[serde(default = "default_true")]
    pub isvisible: bool,
}

/// `POST /api/dev/seed-staff-bulk` — create N staff accounts in one call so the
/// staff-list / directory pagination ACs have enough rows (TS-M4-B1/B3/B5).
///
/// Disabled in production (returns 404). All accounts share one group (created if
/// none is supplied) with `password = Bulk123!`. Returns `{created, staffIds}`.
///
/// @implements FS-031.1 (test infra): bulk staff for pagination.
pub async fn seed_staff_bulk(
    State(state): State<AppState>,
    body: Json<SeedStaffBulkRequest>,
) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Seeding is unavailable").into_response(),
    };

    let count = body.count.clamp(0, 500);
    let prefix = body.prefix.clone().unwrap_or_else(|| "bulkstaff".to_string());

    let dept_id: i32 = match body.dept_id {
        Some(id) => id,
        None => match sqlx::query_scalar("SELECT dept_id FROM department ORDER BY dept_id LIMIT 1")
            .fetch_one(pool)
            .await
        {
            Ok(id) => id,
            Err(_) => return ApiError::internal("No department exists").into_response(),
        },
    };

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let group_id: i32 = match body.group_id {
        Some(id) => id,
        None => match sqlx::query_scalar(
            "INSERT INTO groups (group_name, group_enabled, can_create_tickets)
             VALUES ($1, true, true) RETURNING group_id",
        )
        .bind(format!("Bulk Group {nonce}"))
        .fetch_one(pool)
        .await
        {
            Ok(id) => id,
            Err(e) => {
                tracing::warn!(error = %e, "seed_staff_bulk: group failed");
                return ApiError::internal("Could not create group").into_response();
            }
        },
    };

    let passwd = match ost_core::hash_password("Bulk123!") {
        Ok(h) => h,
        Err(_) => return ApiError::internal("Password hashing failed").into_response(),
    };

    let mut staff_ids: Vec<i32> = Vec::new();
    for n in 0..count {
        let username = format!("{prefix}{nonce}_{n}");
        let firstname = format!("Bulk{n}");
        match sqlx::query_scalar::<_, i32>(
            "INSERT INTO staff (group_id, dept_id, username, firstname, lastname, passwd, \
                                isactive, isvisible)
             VALUES ($1, $2, $3, $4, 'Staff', $5, true, $6)
             ON CONFLICT (username) DO UPDATE SET updated = now()
             RETURNING staff_id",
        )
        .bind(group_id)
        .bind(dept_id)
        .bind(&username)
        .bind(&firstname)
        .bind(&passwd)
        .bind(body.isvisible)
        .fetch_one(pool)
        .await
        {
            Ok(id) => staff_ids.push(id),
            Err(e) => {
                tracing::warn!(error = %e, "seed_staff_bulk: insert failed");
                return ApiError::internal("Could not seed staff").into_response();
            }
        }
    }

    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "created": staff_ids.len(), "staffIds": staff_ids, "groupId": group_id })),
    )
        .into_response()
}

/// `POST /api/dev/reset-staff` — delete non-seed staff accounts, keeping the
/// documented QA logins (`agent`, `agent2`, `admin`), so a staff-CRUD E2E starts
/// from a known roster.
///
/// Disabled in production (returns 404). Nullifies staff-referencing FKs on the
/// to-be-deleted rows first (team leads, help-topic assignees, ticket assignees,
/// closer, department managers, thread authors) so the DELETE cannot trip an FK.
///
/// @implements FS-031 (test infra): staff-roster reset for the staff-CRUD E2E.
pub async fn reset_staff(State(state): State<AppState>) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Reset is unavailable").into_response(),
    };

    const KEEP: [&str; 3] = ["agent", "agent2", "admin"];

    // Collect the ids to remove (everything not in the keep-list).
    let doomed: Vec<i32> = match sqlx::query_scalar(
        "SELECT staff_id FROM staff WHERE username <> ALL($1)",
    )
    .bind(&KEEP[..])
    .fetch_all(pool)
    .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "reset_staff: select failed");
            return ApiError::internal("Could not enumerate staff").into_response();
        }
    };

    if doomed.is_empty() {
        return (StatusCode::OK, Json(serde_json::json!({ "success": true, "deleted": 0 })))
            .into_response();
    }

    // Nullify every staff-referencing FK on the doomed rows, then delete them.
    let nullify: &[&str] = &[
        "UPDATE team SET lead_id = NULL WHERE lead_id = ANY($1)",
        "UPDATE help_topic SET staff_id = NULL WHERE staff_id = ANY($1)",
        "UPDATE ticket SET staff_id = NULL WHERE staff_id = ANY($1)",
        "UPDATE ticket SET closed_by_staff_id = NULL WHERE closed_by_staff_id = ANY($1)",
        "UPDATE ticket_thread SET staff_id = NULL WHERE staff_id = ANY($1)",
        "UPDATE department SET manager_id = NULL WHERE manager_id = ANY($1)",
        "DELETE FROM team_member WHERE staff_id = ANY($1)",
        "DELETE FROM ticket_lock WHERE staff_id = ANY($1)",
        "DELETE FROM staff WHERE staff_id = ANY($1)",
    ];
    for stmt in nullify {
        if let Err(e) = sqlx::query(stmt).bind(&doomed).execute(pool).await {
            tracing::warn!(error = %e, stmt = stmt, "reset_staff: cleanup failed");
            return ApiError::internal("Could not reset staff").into_response();
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({ "success": true, "deleted": doomed.len() })),
    )
        .into_response()
}

/// `POST /api/dev/reset-config` — force-restore the FS-032 config defaults
/// (overwriting admin edits), so a settings-tab E2E can return to a known state.
///
/// Disabled in production (returns 404). Reuses the seed crate's single source
/// of truth ([`tools::restore_config_defaults`]) so the key list never drifts.
///
/// @implements TS-M4-PREP-C (test infra): config-reset path for the settings E2E.
pub async fn reset_config(State(state): State<AppState>) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Reset is unavailable").into_response(),
    };
    if let Err(e) = tools::restore_config_defaults(pool).await {
        tracing::warn!(error = %e, "reset_config: restore failed");
        return ApiError::internal("Could not reset config").into_response();
    }
    (StatusCode::OK, Json(serde_json::json!({ "success": true }))).into_response()
}

// --- TS-M4-A1: seed a site page for the settings Pages/Landing AC --------------

/// Request body for seeding a `page` row.
///
/// @implements TS-M4-A1 (test infra): so the settings Pages/Landing AC
///   (US-M4-A2 / TS-M4-A3 AC-2) is testable without EPIC-M4-F.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedPageRequest {
    /// Page name (unique). Defaults to `Landing Page` (upserted).
    #[serde(default)]
    pub name: Option<String>,
    /// Page type: landing|offline|thank-you|other (defaults to `landing`).
    #[serde(default, rename = "type")]
    pub page_type: Option<String>,
    /// Page body HTML.
    #[serde(default)]
    pub body: Option<String>,
}

/// `POST /api/dev/seed-page` — insert (or upsert by name) a `page` row so the
/// settings Pages/Landing AC has a real page to bind.
///
/// Disabled in production (returns 404). Example: `{"type":"landing"}` → `{"id":1}`.
///
/// @implements TS-M4-A1 (test infra): seed a landing page for the settings E2E.
pub async fn seed_page(State(state): State<AppState>, body: Option<Json<SeedPageRequest>>) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let Json(req) = body.unwrap_or(Json(SeedPageRequest {
        name: None,
        page_type: None,
        body: None,
    }));
    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Seeding is unavailable").into_response(),
    };

    let page_type = req.page_type.as_deref().unwrap_or("landing");
    if !["landing", "offline", "thank-you", "other"].contains(&page_type) {
        return ApiError::validation("type must be one of landing|offline|thank-you|other")
            .into_response();
    }
    let name = req.name.clone().unwrap_or_else(|| "Landing Page".to_string());
    let page_body = req
        .body
        .clone()
        .unwrap_or_else(|| "<h1>Welcome to osTicket</h1>".to_string());

    let id: i32 = match sqlx::query_scalar(
        "INSERT INTO page (name, type, body) VALUES ($1, $2, $3)
         ON CONFLICT (name) DO UPDATE SET type = EXCLUDED.type, body = EXCLUDED.body, updated = now()
         RETURNING id",
    )
    .bind(&name)
    .bind(page_type)
    .bind(&page_body)
    .fetch_one(pool)
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!(error = %e, "seed_page: insert failed");
            return ApiError::internal("Could not seed page").into_response();
        }
    };

    (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response()
}

// --- TS-M4-D1: reset SLA plans to the seeded baseline -------------------------

/// `POST /api/dev/reset-sla` — restore the seeded SLA plans (Default SLA / Standard
/// / Urgent) + the `default_sla_id` binding, and re-home + delete any non-seed
/// plan (dept/topic sla_id → NULL, ticket sla_id → default) so the delete/re-home
/// ACs start from a known state without a full reseed.
///
/// Disabled in production (returns 404).
///
/// @implements FS-032.12 (test infra): SLA-plan reset for the SLA-CRUD E2E.
pub async fn reset_sla(State(state): State<AppState>) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Reset is unavailable").into_response(),
    };

    const SEED: [&str; 3] = ["Default SLA", "Standard", "Urgent"];

    // 1) Upsert the three seed plans (idempotent) so they always exist + active.
    for (name, hours) in [("Default SLA", 48), ("Standard", 24), ("Urgent", 4)] {
        if let Err(e) = sqlx::query(
            "INSERT INTO sla (name, grace_period, isactive, enable_priority_escalation, transient, disable_overdue_alerts)
             VALUES ($1, $2, true, true, false, false)
             ON CONFLICT (name) DO UPDATE
               SET grace_period = EXCLUDED.grace_period, isactive = true, updated = now()",
        )
        .bind(name)
        .bind(hours)
        .execute(pool)
        .await
        {
            tracing::warn!(error = %e, "reset_sla: seed upsert failed");
            return ApiError::internal("Could not restore SLA plans").into_response();
        }
    }

    let default_id: i32 = match sqlx::query_scalar("SELECT id FROM sla WHERE name = 'Default SLA'")
        .fetch_one(pool)
        .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!(error = %e, "reset_sla: default lookup failed");
            return ApiError::internal("Could not resolve the default SLA").into_response();
        }
    };

    // 2) Re-home + delete every non-seed plan.
    let doomed: Vec<i32> = match sqlx::query_scalar("SELECT id FROM sla WHERE name <> ALL($1)")
        .bind(&SEED[..])
        .fetch_all(pool)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "reset_sla: enumerate failed");
            return ApiError::internal("Could not reset SLA plans").into_response();
        }
    };
    for id in doomed {
        let stmts: [(&str, bool); 4] = [
            ("UPDATE department SET sla_id = NULL WHERE sla_id = $1", false),
            ("UPDATE help_topic SET sla_id = NULL WHERE sla_id = $1", false),
            ("UPDATE ticket SET sla_id = $2 WHERE sla_id = $1", true),
            ("DELETE FROM sla WHERE id = $1", false),
        ];
        for (sql, needs_default) in stmts {
            let q = sqlx::query(sql).bind(id);
            let q = if needs_default { q.bind(default_id) } else { q };
            if let Err(e) = q.execute(pool).await {
                tracing::warn!(error = %e, "reset_sla: re-home/delete failed");
                return ApiError::internal("Could not reset SLA plans").into_response();
            }
        }
    }

    // 3) Bind default_sla_id config to the Default SLA.
    if let Err(e) = sqlx::query(
        "INSERT INTO config (key, value) VALUES ('default_sla_id', $1)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated = now()",
    )
    .bind(default_id.to_string())
    .execute(pool)
    .await
    {
        tracing::warn!(error = %e, "reset_sla: default binding failed");
        return ApiError::internal("Could not bind the default SLA").into_response();
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({ "success": true, "default_sla_id": default_id })),
    )
        .into_response()
}

// --- TS-M4-F1: reset site pages + their bindings ------------------------------

/// `POST /api/dev/reset-pages` — delete all `page` rows, reset the `*_page_id`
/// config bindings to `"0"`, and clear `help_topic.page_id` back to `0`, so the
/// bulk delete/disable ACs start from a clean slate.
///
/// Disabled in production (returns 404).
///
/// @implements FS-033.15 (test infra): page reset for the page-CRUD E2E.
pub async fn reset_pages(State(state): State<AppState>) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Reset is unavailable").into_response(),
    };

    let stmts: [&str; 3] = [
        "UPDATE help_topic SET page_id = 0 WHERE page_id <> 0",
        "UPDATE config SET value = '0', updated = now() WHERE key IN ('landing_page_id','offline_page_id','thank-you_page_id')",
        "DELETE FROM page",
    ];
    for sql in stmts {
        if let Err(e) = sqlx::query(sql).execute(pool).await {
            tracing::warn!(error = %e, "reset_pages: statement failed");
            return ApiError::internal("Could not reset pages").into_response();
        }
    }

    (StatusCode::OK, Json(serde_json::json!({ "success": true }))).into_response()
}

// --- TS-M4-H1: reset canned responses to the seeded baseline ------------------

/// `POST /api/dev/reset-canned` — delete every non-seed canned response (its
/// `canned_attachment` rows cascade) and restore the M2 seeded responses, so the
/// canned-CRUD E2E starts from a known state without a full reseed.
///
/// Disabled in production (returns 404).
///
/// @implements FS-022 (test infra): canned reset for the canned-CRUD E2E.
pub async fn reset_canned(State(state): State<AppState>) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Reset is unavailable").into_response(),
    };

    let seed_titles: [&str; 3] = [
        tools::CANNED_TITLE_ACK,
        tools::CANNED_TITLE_POLICY,
        tools::CANNED_TITLE_DISABLED,
    ];

    // Delete non-seed responses (canned_attachment cascades on the FK).
    if let Err(e) = sqlx::query("DELETE FROM canned_response WHERE title <> ALL($1)")
        .bind(&seed_titles[..])
        .execute(pool)
        .await
    {
        tracing::warn!(error = %e, "reset_canned: delete failed");
        return ApiError::internal("Could not reset canned responses").into_response();
    }

    // Restore the seeded responses (idempotent: upsert by title + re-bind blob).
    if let Err(e) = tools::seed_canned(pool).await {
        tracing::warn!(error = %e, "reset_canned: reseed failed");
        return ApiError::internal("Could not restore seeded canned responses").into_response();
    }

    (StatusCode::OK, Json(serde_json::json!({ "success": true }))).into_response()
}

// --- Toggle config endpoint for browser testing --------------------------------

/// Request body for toggling config values.
///
/// Supports toggling `show_answered_tickets` and `show_assigned_tickets` config
/// values for browser testing of queue filtering behavior.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ToggleConfigRequest {
    /// Config key to toggle. Supported keys:
    /// - `show_answered_tickets` (0/1)
    /// - `show_assigned_tickets` (0/1)
    pub key: String,
    /// Value to set (0 or 1 for boolean toggles).
    pub value: i32,
}

/// `POST /api/dev/toggle-config` — toggle config values for testing.
///
/// Supported keys:
/// - `show_answered_tickets` — when 0, excludes answered tickets from open queue
/// - `show_assigned_tickets` — when 0, excludes assigned tickets from open queue
///
/// Disabled in production (returns 404).
pub async fn toggle_config(
    State(state): State<AppState>,
    body: Json<ToggleConfigRequest>,
) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }

    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Config toggle is unavailable").into_response(),
    };

    // Whitelist of allowed config keys for toggle
    let allowed_keys = [
        "show_answered_tickets",
        "show_assigned_tickets",
    ];

    if !allowed_keys.contains(&body.key.as_str()) {
        return ApiError::validation(&format!(
            "Invalid config key '{}'. Allowed keys: {}",
            body.key,
            allowed_keys.join(", ")
        ))
        .into_response();
    }

    let value_str = body.value.to_string();

    if let Err(e) = sqlx::query(
        "INSERT INTO config (key, value) VALUES ($1, $2)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated = now()",
    )
    .bind(&body.key)
    .bind(&value_str)
    .execute(pool)
    .await
    {
        tracing::warn!(error = %e, key = &body.key, "toggle_config: upsert failed");
        return ApiError::internal("Could not toggle config value").into_response();
    }

    let resp = Json(serde_json::json!({
        "success": true,
        "key": body.key,
        "value": body.value
    }));
    (StatusCode::OK, resp).into_response()
}

// --- TS-M4-C1: reset departments to the seeded baseline -----------------------

/// `POST /api/dev/reset-departments` — restore the seeded departments (Support +
/// Sales), rebind `default_dept_id` to Support, and repoint every dependent
/// (`ticket.dept_id` / `staff.dept_id` / `help_topic.dept_id`) off any non-seed
/// department before deleting it, so the department-CRUD E2E starts from a known
/// state without a full reseed.
///
/// Disabled in production (returns 404).
///
/// @implements FS-030.7 (test infra): department reset for the department-CRUD E2E.
pub async fn reset_departments(State(state): State<AppState>) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Reset is unavailable").into_response(),
    };

    // 1) Ensure Support + Sales exist (upsert by unique dept_name).
    let support_id: i32 = match sqlx::query_scalar(
        "INSERT INTO department (dept_name, ispublic) VALUES ($1, true)
         ON CONFLICT (dept_name) DO UPDATE SET updated = now() RETURNING dept_id",
    )
    .bind(tools::DEPT_NAME)
    .fetch_one(pool)
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!(error = %e, "reset_departments: support upsert failed");
            return ApiError::internal("Could not restore departments").into_response();
        }
    };
    if let Err(e) = sqlx::query(
        "INSERT INTO department (dept_name, ispublic) VALUES ($1, true)
         ON CONFLICT (dept_name) DO UPDATE SET updated = now()",
    )
    .bind(tools::DEPT_NAME_SALES)
    .execute(pool)
    .await
    {
        tracing::warn!(error = %e, "reset_departments: sales upsert failed");
        return ApiError::internal("Could not restore departments").into_response();
    }

    // 2) Bind default_dept_id → Support.
    if let Err(e) = sqlx::query(
        "INSERT INTO config (key, value) VALUES ($1, $2)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated = now()",
    )
    .bind(tools::CFG_DEFAULT_DEPT_ID)
    .bind(support_id.to_string())
    .execute(pool)
    .await
    {
        tracing::warn!(error = %e, "reset_departments: default binding failed");
        return ApiError::internal("Could not bind the default department").into_response();
    }

    // 3) Repoint dependents off every non-seed department, then delete them.
    let seed: [&str; 2] = [tools::DEPT_NAME, tools::DEPT_NAME_SALES];
    let doomed: Vec<i32> =
        match sqlx::query_scalar("SELECT dept_id FROM department WHERE dept_name <> ALL($1)")
            .bind(&seed[..])
            .fetch_all(pool)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "reset_departments: enumerate failed");
                return ApiError::internal("Could not reset departments").into_response();
            }
        };
    if !doomed.is_empty() {
        let stmts: [&str; 4] = [
            "UPDATE ticket SET dept_id = $2 WHERE dept_id = ANY($1)",
            "UPDATE staff SET dept_id = $2 WHERE dept_id = ANY($1)",
            "UPDATE help_topic SET dept_id = $2 WHERE dept_id = ANY($1)",
            "DELETE FROM department WHERE dept_id = ANY($1)",
        ];
        for (i, sql) in stmts.iter().enumerate() {
            let q = sqlx::query(sql).bind(&doomed);
            let q = if i < 3 { q.bind(support_id) } else { q };
            if let Err(e) = q.execute(pool).await {
                tracing::warn!(error = %e, "reset_departments: repoint/delete failed");
                return ApiError::internal("Could not reset departments").into_response();
            }
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({ "success": true, "default_dept_id": support_id })),
    )
        .into_response()
}

// --- TS-M4-C3: reset teams to the seeded baseline -----------------------------

/// `POST /api/dev/reset-teams` — release non-seed teams' associations
/// (`ticket.team_id=0`, `help_topic.team_id=NULL`), delete every non-seed team,
/// and restore the "Tier 2" team with `agent` as its member + lead.
///
/// Disabled in production (returns 404).
///
/// @implements FS-030.12 (test infra): team reset for the team-CRUD E2E.
pub async fn reset_teams(State(state): State<AppState>) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Reset is unavailable").into_response(),
    };

    // 1) Release + delete every non-seed team (Tier 2 is the only seed team).
    let doomed: Vec<i32> = match sqlx::query_scalar("SELECT team_id FROM team WHERE name <> $1")
        .bind(tools::TEAM_NAME)
        .fetch_all(pool)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "reset_teams: enumerate failed");
            return ApiError::internal("Could not reset teams").into_response();
        }
    };
    if !doomed.is_empty() {
        let stmts: [&str; 3] = [
            "UPDATE ticket SET team_id = 0 WHERE team_id = ANY($1)",
            "UPDATE help_topic SET team_id = NULL WHERE team_id = ANY($1)",
            "DELETE FROM team WHERE team_id = ANY($1)", // team_member cascades
        ];
        for sql in stmts {
            if let Err(e) = sqlx::query(sql).bind(&doomed).execute(pool).await {
                tracing::warn!(error = %e, "reset_teams: release/delete failed");
                return ApiError::internal("Could not reset teams").into_response();
            }
        }
    }

    // 2) Restore Tier 2 + agent membership/lead.
    let agent_id: Option<i32> = sqlx::query_scalar("SELECT staff_id FROM staff WHERE username = $1")
        .bind(tools::STAFF_USERNAME)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
    let team_id: i32 = match sqlx::query_scalar(
        "INSERT INTO team (name, isenabled, lead_id) VALUES ($1, true, $2)
         ON CONFLICT (name) DO UPDATE SET isenabled = true, lead_id = EXCLUDED.lead_id, updated = now()
         RETURNING team_id",
    )
    .bind(tools::TEAM_NAME)
    .bind(agent_id)
    .fetch_one(pool)
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!(error = %e, "reset_teams: Tier 2 upsert failed");
            return ApiError::internal("Could not restore the seed team").into_response();
        }
    };
    if let Some(sid) = agent_id {
        if let Err(e) = sqlx::query(
            "INSERT INTO team_member (team_id, staff_id) VALUES ($1, $2)
             ON CONFLICT (team_id, staff_id) DO NOTHING",
        )
        .bind(team_id)
        .bind(sid)
        .execute(pool)
        .await
        {
            tracing::warn!(error = %e, "reset_teams: membership restore failed");
            return ApiError::internal("Could not restore team membership").into_response();
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({ "success": true, "team_id": team_id })),
    )
        .into_response()
}

// --- TS-M4-C5: reset help topics to the seeded baseline -----------------------

/// `POST /api/dev/reset-help-topics` — clear `ticket.topic_id=0` for tickets on
/// any non-seed topic, delete every non-seed topic, and restore the seeded
/// "General" (→ Support) and "Billing" (→ Sales, Urgent SLA) topics.
///
/// Disabled in production (returns 404).
///
/// @implements FS-030.17 (test infra): help-topic reset for the topic-CRUD E2E.
pub async fn reset_help_topics(State(state): State<AppState>) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Reset is unavailable").into_response(),
    };

    // 1) Clear ticket refs + delete every non-seed topic.
    let seed: [&str; 2] = [tools::TOPIC_GENERAL, tools::TOPIC_BILLING];
    let doomed: Vec<i32> =
        match sqlx::query_scalar("SELECT topic_id FROM help_topic WHERE topic <> ALL($1)")
            .bind(&seed[..])
            .fetch_all(pool)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "reset_help_topics: enumerate failed");
                return ApiError::internal("Could not reset help topics").into_response();
            }
        };
    if !doomed.is_empty() {
        let stmts: [&str; 2] = [
            "UPDATE ticket SET topic_id = 0 WHERE topic_id = ANY($1)",
            "DELETE FROM help_topic WHERE topic_id = ANY($1)",
        ];
        for sql in stmts {
            if let Err(e) = sqlx::query(sql).bind(&doomed).execute(pool).await {
                tracing::warn!(error = %e, "reset_help_topics: clear/delete failed");
                return ApiError::internal("Could not reset help topics").into_response();
            }
        }
    }

    // 2) Restore General (Support) + Billing (Sales, Urgent SLA).
    let support_id: Option<i32> = sqlx::query_scalar("SELECT dept_id FROM department WHERE dept_name = $1")
        .bind(tools::DEPT_NAME)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
    let sales_id: Option<i32> = sqlx::query_scalar("SELECT dept_id FROM department WHERE dept_name = $1")
        .bind(tools::DEPT_NAME_SALES)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
    let urgent_id: Option<i32> = sqlx::query_scalar("SELECT id FROM sla WHERE name = 'Urgent'")
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

    if let Some(dept) = support_id {
        if let Err(e) = sqlx::query(
            "INSERT INTO help_topic
               (topic, topic_pid, isactive, ispublic, noautoresp, priority_id, dept_id, sla_id, page_id, sort)
             VALUES ($1, 0, true, true, false, 2, $2, NULL, 0, 1)
             ON CONFLICT (topic, topic_pid) DO UPDATE
               SET isactive = true, dept_id = EXCLUDED.dept_id, sla_id = NULL, updated = now()",
        )
        .bind(tools::TOPIC_GENERAL)
        .bind(dept)
        .execute(pool)
        .await
        {
            tracing::warn!(error = %e, "reset_help_topics: General restore failed");
            return ApiError::internal("Could not restore seed topics").into_response();
        }
    }
    if let Some(dept) = sales_id {
        if let Err(e) = sqlx::query(
            "INSERT INTO help_topic
               (topic, topic_pid, isactive, ispublic, noautoresp, priority_id, dept_id, sla_id, page_id, sort)
             VALUES ($1, 0, true, true, false, 2, $2, $3, 0, 2)
             ON CONFLICT (topic, topic_pid) DO UPDATE
               SET isactive = true, dept_id = EXCLUDED.dept_id, sla_id = EXCLUDED.sla_id, updated = now()",
        )
        .bind(tools::TOPIC_BILLING)
        .bind(dept)
        .bind(urgent_id)
        .execute(pool)
        .await
        {
            tracing::warn!(error = %e, "reset_help_topics: Billing restore failed");
            return ApiError::internal("Could not restore seed topics").into_response();
        }
    }

    (StatusCode::OK, Json(serde_json::json!({ "success": true }))).into_response()
}

// --- TS-M4-E1: reset FAQ categories -------------------------------------------

/// `POST /api/dev/reset-faq-categories` — delete every `faq_category` row so the
/// FAQ-category E2E starts from an empty slate. No articles exist yet (M7), so a
/// plain delete is sufficient.
///
/// Disabled in production (returns 404).
///
/// @implements FS-032.16 (test infra): FAQ-category reset for the FAQ-category E2E.
pub async fn reset_faq_categories(State(state): State<AppState>) -> Response {
    if !state.app_env.dev_endpoints_enabled() {
        return ApiError::not_found("Not found").into_response();
    }
    let pool = match state.pool.as_ref() {
        Some(p) => p,
        None => return ApiError::internal("Reset is unavailable").into_response(),
    };

    if let Err(e) = sqlx::query("DELETE FROM faq_category").execute(pool).await {
        tracing::warn!(error = %e, "reset_faq_categories: delete failed");
        return ApiError::internal("Could not reset FAQ categories").into_response();
    }

    (StatusCode::OK, Json(serde_json::json!({ "success": true }))).into_response()
}
