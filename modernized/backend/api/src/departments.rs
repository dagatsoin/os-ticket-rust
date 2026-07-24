//! Admin department management (TS-M4-C1) — FS-030.5/.6/.7.
//!
//! Admin-gated CRUD over the `department` table with the required email/template
//! bindings, optional SLA/manager, group-access full-replace sync, the default-
//! department protections, and the delete re-home contract.
//!
//! Route shapes (all under `/api/staff/admin`, admin-gated per-handler; mutating
//! routes CSRF-enforced):
//! * `GET  /departments` — list (is_default flags the `default_dept_id` one).
//! * `GET  /departments/form-options` — select sources for the dept form.
//! * `GET  /departments/:id` — one department + its `group_ids`.
//! * `POST /departments` — create (email + template REQUIRED).
//! * `PUT  /departments/:id` — edit (default cannot be made private).
//! * `POST /departments/mass` — delete / makepublic / makeprivate.
//! * `DELETE /departments/:id` — delete + re-home tickets/topics (default protected).
//!
//! @implements FS-030.5: department list + create.
//! @implements FS-030.6: department edit + group-access sync.
//! @implements FS-030.7: department delete re-home + default protection.
//! @implements BS-030-01: name required, ≥4 chars, unique.
//! @implements BS-030-02: department email binding required.
//! @implements BS-030-03: department template binding required.
//! @implements BS-030-04: the system default department cannot be private.
//! @implements BS-030-05: the default department is excluded from delete/disable.
//! @implements BS-030-06: delete refused while home staff remain.
//! @implements BS-030-07: delete re-homes tickets + topics only; drops group access.
//! @implements BS-030-08: group-access full-replace (inverted sync) on save.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use ost_core::ApiError;

use crate::auth::gate::require_admin;
use crate::auth::realm::StaffSession;
use crate::auth::StaffCsrf;
use crate::config_keys::read_config;
use crate::state::AppState;

/// Config key naming the system default department (tickets/topics re-home here).
pub const CFG_DEFAULT_DEPT_ID: &str = "default_dept_id";

fn pool(state: &AppState) -> Result<&sqlx::postgres::PgPool, ApiError> {
    state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Departments are unavailable"))
}

/// Read the `default_dept_id` config value as an integer id (`None` when unset).
async fn default_dept_id(pool: &sqlx::postgres::PgPool) -> Result<Option<i32>, ApiError> {
    Ok(read_config(pool, CFG_DEFAULT_DEPT_ID)
        .await?
        .and_then(|v| v.trim().parse::<i32>().ok()))
}

// ---------------------------------------------------------------------------
// GET /api/staff/admin/departments — list.
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
pub struct DeptListParams {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub order: Option<String>,
}

fn dept_sort_expr(sort: Option<&str>) -> &'static str {
    match sort.unwrap_or("name") {
        "status" | "ispublic" | "type" => "d.ispublic",
        "users" => "user_count",
        "updated" => "d.updated",
        "created" => "d.created",
        _ => "d.dept_name",
    }
}

/// `GET /api/staff/admin/departments` — every department (admin only).
///
/// @implements FS-030.5: department list with default flag, manager, SLA, users.
pub async fn list_departments(
    State(state): State<AppState>,
    session: StaffSession,
    Query(params): Query<DeptListParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    let order = match params.order.as_deref().map(|s| s.to_uppercase()) {
        Some(ref o) if o == "DESC" => "DESC",
        _ => "ASC",
    };
    let order_by = format!("{} {order}, d.dept_id ASC", dept_sort_expr(params.sort.as_deref()));

    let default_id = default_dept_id(pool).await?;

    let sql = format!(
        "SELECT d.dept_id, d.dept_name, d.ispublic, d.manager_id, \
                COALESCE(NULLIF(TRIM(m.firstname || ' ' || m.lastname), ''), m.username, '') AS manager_name, \
                d.email_id, d.tpl_id, d.sla_id, COALESCE(s.name, '') AS sla_name, \
                COALESCE(uc.user_count, 0) AS user_count, \
                to_char(d.updated, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated \
         FROM department d \
         LEFT JOIN staff m ON m.staff_id = d.manager_id \
         LEFT JOIN sla s ON s.id = d.sla_id \
         LEFT JOIN (SELECT dept_id, COUNT(*) AS user_count FROM staff GROUP BY dept_id) uc \
                ON uc.dept_id = d.dept_id \
         ORDER BY {order_by}"
    );

    // (id, name, ispublic, manager_id, manager_name, email_id, tpl_id, sla_id, sla_name, user_count, updated)
    type Row = (
        i32,
        String,
        bool,
        Option<i32>,
        String,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        String,
        i64,
        Option<String>,
    );
    let rows: Vec<Row> = sqlx::query_as(&sql)
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::internal("Department list failed"))?;

    let out: Vec<Value> = rows
        .into_iter()
        .map(
            |(id, name, ispublic, manager_id, manager_name, email_id, tpl_id, sla_id, sla_name, user_count, updated)| {
                json!({
                    "id": id,
                    "name": name,
                    "ispublic": ispublic,
                    "is_default": Some(id) == default_id,
                    "manager_id": manager_id,
                    "manager_name": manager_name,
                    "email_id": email_id,
                    "tpl_id": tpl_id,
                    "sla_id": sla_id,
                    "sla_name": sla_name,
                    "user_count": user_count,
                    "updated": updated,
                })
            },
        )
        .collect();

    Ok(Json(json!(out)))
}

// ---------------------------------------------------------------------------
// GET /api/staff/admin/departments/form-options — select sources.
// ---------------------------------------------------------------------------

/// `GET /api/staff/admin/departments/form-options` — the source lists for the
/// department form's Email / Template / SLA / manager / group-access selects.
///
/// The `email_accounts` and `template_groups` reads are NET-NEW (no prior
/// endpoint exposes them); `sla` / `staff` / `groups` are surfaced here too so
/// the form has a single options payload.
///
/// @implements FS-030.5: department form option sources (email/template net-new).
/// @implements BS-030-02: department Email select source (email_account rows).
/// @implements BS-030-03: department Template select source (template_group rows).
pub async fn form_options(
    State(state): State<AppState>,
    session: StaffSession,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    // email_accounts: [{id, email, name}]
    type EmailRow = (i32, String, String);
    let email_rows: Vec<EmailRow> = sqlx::query_as(
        "SELECT id, email, name FROM email_account WHERE active = true ORDER BY email ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Email account lookup failed"))?;
    let email_accounts: Vec<Value> = email_rows
        .into_iter()
        .map(|(id, email, name)| json!({ "id": id, "email": email, "name": name }))
        .collect();

    // template_groups: [{id, name}]
    type NameRow = (i32, String);
    let tpl_rows: Vec<NameRow> = sqlx::query_as(
        "SELECT id, name FROM template_group WHERE isactive = true ORDER BY name ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Template group lookup failed"))?;
    let template_groups: Vec<Value> = tpl_rows
        .into_iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();

    // sla: [{id, name}]
    let sla_rows: Vec<NameRow> =
        sqlx::query_as("SELECT id, name FROM sla WHERE isactive = true ORDER BY name ASC")
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::internal("SLA lookup failed"))?;
    let sla: Vec<Value> = sla_rows
        .into_iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();

    // staff: [{id, name}]
    let staff_rows: Vec<NameRow> = sqlx::query_as(
        "SELECT staff_id, COALESCE(NULLIF(TRIM(firstname || ' ' || lastname), ''), username) AS name \
         FROM staff WHERE isactive = true ORDER BY name ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Staff lookup failed"))?;
    let staff: Vec<Value> = staff_rows
        .into_iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();

    // groups: [{id, name}]
    let group_rows: Vec<NameRow> =
        sqlx::query_as("SELECT group_id, group_name FROM groups ORDER BY group_name ASC")
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::internal("Group lookup failed"))?;
    let groups: Vec<Value> = group_rows
        .into_iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();

    Ok(Json(json!({
        "email_accounts": email_accounts,
        "template_groups": template_groups,
        "sla": sla,
        "staff": staff,
        "groups": groups,
    })))
}

// ---------------------------------------------------------------------------
// GET /api/staff/admin/departments/:id — detail.
// ---------------------------------------------------------------------------

/// `GET /api/staff/admin/departments/:id` — one department + `group_ids`.
///
/// @implements FS-030.6: department detail (full row + group-access set).
pub async fn get_department(
    State(state): State<AppState>,
    session: StaffSession,
    Path(id): Path<i32>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    // (id, name, ispublic, manager_id, email_id, tpl_id, sla_id, autoresp_email_id,
    //  ticket_auto_response, message_auto_response, dept_signature, group_membership, updated)
    type Row = (
        i32,
        String,
        bool,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        bool,
        bool,
        String,
        i32,
        Option<String>,
    );
    let row: Option<Row> = sqlx::query_as(
        "SELECT dept_id, dept_name, ispublic, manager_id, email_id, tpl_id, sla_id, \
                autoresp_email_id, ticket_auto_response, message_auto_response, \
                dept_signature, group_membership, \
                to_char(updated, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated \
         FROM department WHERE dept_id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal("Department lookup failed"))?;

    let row = row.ok_or_else(|| ApiError::not_found("Department not found"))?;

    let group_ids: Vec<i32> = sqlx::query_scalar(
        "SELECT group_id FROM group_dept_access WHERE dept_id = $1 ORDER BY group_id",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Group-access lookup failed"))?;

    let default_id = default_dept_id(pool).await?;

    Ok(Json(json!({
        "id": row.0,
        "name": row.1,
        "ispublic": row.2,
        "is_default": Some(row.0) == default_id,
        "manager_id": row.3,
        "email_id": row.4,
        "tpl_id": row.5,
        "sla_id": row.6,
        "autoresp_email_id": row.7,
        "ticket_auto_response": row.8,
        "message_auto_response": row.9,
        "dept_signature": row.10,
        "group_membership": row.11,
        "group_ids": group_ids,
        "updated": row.12,
    })))
}

// ---------------------------------------------------------------------------
// POST / PUT — create / edit.
// ---------------------------------------------------------------------------

/// Create/edit department request body (camelCase).
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeptWriteRequest {
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_true")]
    pub ispublic: bool,
    #[serde(default)]
    pub email_id: Option<i32>,
    #[serde(default)]
    pub tpl_id: Option<i32>,
    #[serde(default)]
    pub sla_id: Option<i32>,
    #[serde(default)]
    pub manager_id: Option<i32>,
    #[serde(default)]
    pub group_membership: Option<i32>,
    #[serde(default = "default_true")]
    pub ticket_auto_response: bool,
    #[serde(default = "default_true")]
    pub message_auto_response: bool,
    #[serde(default)]
    pub autoresp_email_id: Option<i32>,
    #[serde(default)]
    pub dept_signature: Option<String>,
    #[serde(default)]
    pub group_ids: Vec<i32>,
}

fn default_true() -> bool {
    true
}

/// Validated + trimmed department write fields.
#[derive(Debug)]
struct DeptFields {
    name: String,
    email_id: i32,
    tpl_id: i32,
}

/// Validate the required department fields (BS-030-01/02/03). Name ≥4 chars;
/// email + template both required (distinct per-field messages).
fn validate_dept_fields(body: &DeptWriteRequest) -> Result<DeptFields, ApiError> {
    let mut err = ApiError::validation("Please correct the errors below");
    let name = body.name.trim().to_string();
    if name.is_empty() {
        err = err.with_field("name", "Name required");
    } else if name.chars().count() < 4 {
        err = err.with_field("name", "Name must be at least 4 chars.");
    }
    // A zero id is treated as "not selected" (the form's empty-option sentinel).
    let email_id = body.email_id.filter(|&v| v > 0);
    if email_id.is_none() {
        err = err.with_field("emailId", "Email selection required");
    }
    let tpl_id = body.tpl_id.filter(|&v| v > 0);
    if tpl_id.is_none() {
        err = err.with_field("tplId", "Template selection required");
    }
    if !err.fields.is_empty() {
        return Err(err);
    }
    Ok(DeptFields {
        name,
        email_id: email_id.unwrap(),
        tpl_id: tpl_id.unwrap(),
    })
}

/// Map a unique-violation on `department_dept_name_key` (23505) to a 422 keyed on
/// `name`. Any other DB error becomes a 500.
fn map_dept_write_error(e: sqlx::Error, context: &'static str) -> ApiError {
    if let sqlx::Error::Database(db) = &e {
        if db.code().as_deref() == Some("23505")
            && db.constraint() == Some("department_dept_name_key")
        {
            return ApiError::validation("A department with this name already exists")
                .with_field("name", "A department with this name already exists");
        }
    }
    tracing::warn!(error = %e, context, "department write failed");
    ApiError::internal("Could not save the department")
}

/// Replace a department's group-access set (BS-030-08 inverted full-replace):
/// `DELETE FROM group_dept_access WHERE dept_id = $1` then one insert per group.
async fn replace_group_access(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    dept_id: i32,
    group_ids: &[i32],
) -> Result<(), ApiError> {
    sqlx::query("DELETE FROM group_dept_access WHERE dept_id = $1")
        .bind(dept_id)
        .execute(&mut **tx)
        .await
        .map_err(|_| ApiError::internal("Group-access reset failed"))?;
    let mut seen = std::collections::HashSet::new();
    for &group_id in group_ids {
        if !seen.insert(group_id) {
            continue;
        }
        sqlx::query(
            "INSERT INTO group_dept_access (group_id, dept_id) VALUES ($1, $2) \
             ON CONFLICT (group_id, dept_id) DO NOTHING",
        )
        .bind(group_id)
        .bind(dept_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "replace_group_access: insert failed");
            ApiError::internal("Group-access update failed")
        })?;
    }
    Ok(())
}

/// `POST /api/staff/admin/departments` — create (admin + CSRF).
///
/// @implements FS-030.5: create with validation + group-access sync.
/// @implements BS-030-08: group-access full-replace.
pub async fn create_department(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Json(body): Json<DeptWriteRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    let f = validate_dept_fields(&body)?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal("Department create failed"))?;

    let id: i32 = sqlx::query_scalar(
        "INSERT INTO department \
            (dept_name, ispublic, email_id, tpl_id, sla_id, manager_id, group_membership, \
             ticket_auto_response, message_auto_response, autoresp_email_id, dept_signature) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING dept_id",
    )
    .bind(&f.name)
    .bind(body.ispublic)
    .bind(f.email_id)
    .bind(f.tpl_id)
    .bind(body.sla_id.filter(|&v| v > 0))
    .bind(body.manager_id.filter(|&v| v > 0))
    .bind(body.group_membership.unwrap_or(0))
    .bind(body.ticket_auto_response)
    .bind(body.message_auto_response)
    .bind(body.autoresp_email_id.filter(|&v| v > 0))
    .bind(body.dept_signature.as_deref().unwrap_or(""))
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| map_dept_write_error(e, "create_department"))?;

    replace_group_access(&mut tx, id, &body.group_ids).await?;

    tx.commit()
        .await
        .map_err(|_| ApiError::internal("Department create failed"))?;

    Ok(Json(json!({ "id": id })))
}

/// `PUT /api/staff/admin/departments/:id` — edit (admin + CSRF). 404 if absent.
///
/// Refuses making the system default department private (BS-030-04). A no-op
/// re-save reports success (KL-030-11 modernised).
///
/// @implements FS-030.6: edit with validation + group-access sync.
/// @implements BS-030-04: the default department cannot be made private.
/// @implements BS-030-08: group-access full-replace.
pub async fn update_department(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i32>,
    Json(body): Json<DeptWriteRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    let exists: Option<i32> = sqlx::query_scalar("SELECT dept_id FROM department WHERE dept_id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("Department lookup failed"))?;
    if exists.is_none() {
        return Err(ApiError::not_found("Department not found"));
    }

    // BS-030-04: the system default department may not be made private.
    if !body.ispublic && default_dept_id(pool).await? == Some(id) {
        return Err(ApiError::validation(
            "System default department cannot be private",
        ));
    }

    let f = validate_dept_fields(&body)?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal("Department edit failed"))?;

    sqlx::query(
        "UPDATE department SET dept_name = $2, ispublic = $3, email_id = $4, tpl_id = $5, \
                sla_id = $6, manager_id = $7, group_membership = $8, ticket_auto_response = $9, \
                message_auto_response = $10, autoresp_email_id = $11, dept_signature = $12, \
                updated = now() \
         WHERE dept_id = $1",
    )
    .bind(id)
    .bind(&f.name)
    .bind(body.ispublic)
    .bind(f.email_id)
    .bind(f.tpl_id)
    .bind(body.sla_id.filter(|&v| v > 0))
    .bind(body.manager_id.filter(|&v| v > 0))
    .bind(body.group_membership.unwrap_or(0))
    .bind(body.ticket_auto_response)
    .bind(body.message_auto_response)
    .bind(body.autoresp_email_id.filter(|&v| v > 0))
    .bind(body.dept_signature.as_deref().unwrap_or(""))
    .execute(&mut *tx)
    .await
    .map_err(|e| map_dept_write_error(e, "update_department"))?;

    replace_group_access(&mut tx, id, &body.group_ids).await?;

    tx.commit()
        .await
        .map_err(|_| ApiError::internal("Department edit failed"))?;

    Ok(Json(json!({ "id": id })))
}

// ---------------------------------------------------------------------------
// DELETE /api/staff/admin/departments/:id — delete + re-home.
// ---------------------------------------------------------------------------

/// Re-home a department's tickets + topics to `default_id`, drop its group access,
/// then delete it, all in one transaction. The caller has verified `id` is not the
/// default and that no home staff remain.
async fn delete_dept_rehome(
    pool: &sqlx::postgres::PgPool,
    id: i32,
    default_id: i32,
) -> Result<(), ApiError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal("Department delete failed"))?;

    sqlx::query("UPDATE ticket SET dept_id = $2, updated = now() WHERE dept_id = $1")
        .bind(id)
        .bind(default_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("Department delete failed (ticket re-home)"))?;
    sqlx::query("UPDATE help_topic SET dept_id = $2, updated = now() WHERE dept_id = $1")
        .bind(id)
        .bind(default_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("Department delete failed (topic re-home)"))?;
    sqlx::query("DELETE FROM group_dept_access WHERE dept_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("Department delete failed (group access)"))?;
    sqlx::query("DELETE FROM department WHERE dept_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("Department delete failed"))?;

    tx.commit()
        .await
        .map_err(|_| ApiError::internal("Department delete failed"))?;
    Ok(())
}

/// Count home staff (`staff.dept_id = id`) for the delete guard.
async fn home_staff_count(
    pool: &sqlx::postgres::PgPool,
    ids: &[i32],
) -> Result<i64, ApiError> {
    sqlx::query_scalar("SELECT COUNT(*) FROM staff WHERE dept_id = ANY($1)")
        .bind(ids)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal("Staff lookup failed"))
}

/// `DELETE /api/staff/admin/departments/:id` — delete + re-home tickets/topics.
///
/// Refuses the default department (or when no default is configured), and refuses
/// while any home staff remain.
///
/// @implements FS-030.7: delete re-home + default protection.
/// @implements BS-030-05: the default department cannot be deleted.
/// @implements BS-030-06: delete refused while home staff remain.
/// @implements BS-030-07: re-home tickets + topics; drop group access.
pub async fn delete_department(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i32>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    let exists: Option<i32> = sqlx::query_scalar("SELECT dept_id FROM department WHERE dept_id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("Department lookup failed"))?;
    if exists.is_none() {
        return Err(ApiError::not_found("Department not found"));
    }

    let default = default_dept_id(pool).await?;
    match default {
        None => {
            return Err(ApiError::validation(
                "No default department is configured, so a department cannot be deleted.",
            ));
        }
        Some(d) if d == id => {
            return Err(ApiError::validation("The default department cannot be deleted."));
        }
        _ => {}
    }
    let default = default.expect("checked non-None above");

    // BS-030-06: refuse while any home staff remain.
    if home_staff_count(pool, &[id]).await? > 0 {
        return Err(ApiError::validation(
            "Department has staff members; reassign them first",
        ));
    }

    delete_dept_rehome(pool, id, default).await?;

    Ok(Json(json!({ "affected": 1, "message": "Department deleted" })))
}

// ---------------------------------------------------------------------------
// POST /api/staff/admin/departments/mass — delete / makepublic / makeprivate.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassDeptRequest {
    pub action: String,
    #[serde(default)]
    pub ids: Vec<i32>,
}

/// `POST /api/staff/admin/departments/mass` — delete / makepublic / makeprivate.
///
/// Mass delete refuses the whole batch if any selected department has home staff;
/// the default department is never deleted or made private.
///
/// @implements FS-030.5: department mass actions.
/// @implements BS-030-04: makeprivate never affects the default department.
/// @implements BS-030-05: the default department is excluded from delete.
/// @implements BS-030-06: mass delete refuses the whole batch on any home staff.
pub async fn mass_departments(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Json(body): Json<MassDeptRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    let action = body.action.trim().to_lowercase();
    if !matches!(action.as_str(), "delete" | "makepublic" | "makeprivate") {
        return Err(ApiError::validation("Unknown mass action"));
    }
    if body.ids.is_empty() {
        return Err(ApiError::validation("You must select at least one department."));
    }
    let ids: Vec<i32> = {
        let mut v = body.ids.clone();
        v.sort_unstable();
        v.dedup();
        v
    };
    let requested = ids.len() as i64;
    let default = default_dept_id(pool).await?;

    let (affected, message) = match action.as_str() {
        "makepublic" => {
            let n = sqlx::query("UPDATE department SET ispublic = true, updated = now() WHERE dept_id = ANY($1)")
                .bind(&ids)
                .execute(pool)
                .await
                .map_err(|_| ApiError::internal("Update failed"))?
                .rows_affected() as i64;
            (n, "Selected departments made public".to_string())
        }
        "makeprivate" => {
            // BS-030-04: never make the default department private.
            let n = match default {
                Some(d) => {
                    sqlx::query("UPDATE department SET ispublic = false, updated = now() WHERE dept_id = ANY($1) AND dept_id <> $2")
                        .bind(&ids)
                        .bind(d)
                        .execute(pool)
                        .await
                }
                None => {
                    sqlx::query("UPDATE department SET ispublic = false, updated = now() WHERE dept_id = ANY($1)")
                        .bind(&ids)
                        .execute(pool)
                        .await
                }
            }
            .map_err(|_| ApiError::internal("Update failed"))?
            .rows_affected() as i64;
            (n, "Selected departments made internal".to_string())
        }
        "delete" => {
            let Some(default) = default else {
                return Ok(Json(json!({
                    "affected": 0,
                    "message": "No default department is configured, so departments cannot be deleted."
                })));
            };
            // BS-030-06: refuse the whole batch if any selected dept has home staff.
            if home_staff_count(pool, &ids).await? > 0 {
                return Ok(Json(json!({
                    "affected": 0,
                    "message": "Department has staff members; reassign them first"
                })));
            }
            let mut deleted = 0i64;
            for &id in &ids {
                if id == default {
                    continue;
                }
                delete_dept_rehome(pool, id, default).await?;
                deleted += 1;
            }
            let msg = if deleted == requested {
                "Selected departments deleted successfully".to_string()
            } else if deleted == 0 {
                "Unable to delete selected departments (the default department cannot be deleted)".to_string()
            } else {
                format!("{deleted} of {requested} selected departments deleted")
            };
            (deleted, msg)
        }
        _ => unreachable!(),
    };

    Ok(Json(json!({ "affected": affected, "message": message })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_expr_maps_keys() {
        assert_eq!(dept_sort_expr(None), "d.dept_name");
        assert_eq!(dept_sort_expr(Some("users")), "user_count");
        assert_eq!(dept_sort_expr(Some("updated")), "d.updated");
        assert_eq!(dept_sort_expr(Some("status")), "d.ispublic");
    }

    #[test]
    fn validate_requires_name_email_template() {
        let err = validate_dept_fields(&DeptWriteRequest::default()).unwrap_err();
        assert!(err.fields.contains_key("name"));
        assert!(err.fields.contains_key("emailId"));
        assert!(err.fields.contains_key("tplId"));
    }

    #[test]
    fn validate_rejects_short_name() {
        let body = DeptWriteRequest {
            name: "Ops".into(),
            email_id: Some(1),
            tpl_id: Some(1),
            ..Default::default()
        };
        let err = validate_dept_fields(&body).unwrap_err();
        assert!(err.fields.contains_key("name"));
        assert!(!err.fields.contains_key("emailId"));
    }

    #[test]
    fn validate_accepts_valid() {
        let body = DeptWriteRequest {
            name: "  Support  ".into(),
            email_id: Some(3),
            tpl_id: Some(2),
            ..Default::default()
        };
        let f = validate_dept_fields(&body).unwrap();
        assert_eq!(f.name, "Support");
        assert_eq!(f.email_id, 3);
        assert_eq!(f.tpl_id, 2);
    }

    #[test]
    fn zero_email_id_is_not_selected() {
        let body = DeptWriteRequest {
            name: "Support".into(),
            email_id: Some(0),
            tpl_id: Some(2),
            ..Default::default()
        };
        assert!(validate_dept_fields(&body).unwrap_err().fields.contains_key("emailId"));
    }
}
