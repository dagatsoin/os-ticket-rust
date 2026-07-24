//! Admin SLA-plan management + read-only priority list (TS-M4-D1) — FS-032.9/.10/.12.
//!
//! Admin-gated CRUD over the `sla` table with the FS-032.12 deletion contract
//! (re-home dependents, protect the default plan) plus the fixed BS-032.13
//! priority set as a read-only endpoint.
//!
//! Route shapes (all under `/api/staff/admin`, admin-gated per-handler; mutating
//! routes CSRF-enforced):
//! * `GET  /sla` — list every plan (`is_default` flags the `default_sla_id` one).
//! * `POST /sla` — create (name required + unique, grace_period positive int).
//! * `PUT  /sla/:id` — edit (404 when absent).
//! * `POST /sla/mass` — activate / disable / delete (partial "N of M" messaging).
//! * `DELETE /sla/:id` — delete + re-home dependents in one txn (FS-032.12).
//! * `GET  /priorities` — the fixed four priorities, `ORDER BY urgency DESC`.
//!
//! @implements FS-032.9: SLA plan list.
//! @implements FS-032.10: SLA create/edit validation (name + grace period).
//! @implements FS-032.12: SLA delete re-homes dependents; default is protected.
//! @implements BS-032.13: fixed priority set, read-only, ranked by urgency.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use ost_core::{log, ApiError, LogType};

use crate::auth::gate::require_admin;
use crate::auth::realm::StaffSession;
use crate::auth::StaffCsrf;
use crate::config_keys::read_config;
use crate::logs::client_ip;
use crate::state::AppState;

/// Config key naming the default SLA plan (tickets re-home here on delete).
pub const CFG_DEFAULT_SLA_ID: &str = "default_sla_id";

/// Read the `default_sla_id` config value as an integer id (`None` when unset or
/// non-numeric).
async fn default_sla_id(pool: &sqlx::postgres::PgPool) -> Result<Option<i32>, ApiError> {
    Ok(read_config(pool, CFG_DEFAULT_SLA_ID)
        .await?
        .and_then(|v| v.trim().parse::<i32>().ok()))
}

// ---------------------------------------------------------------------------
// GET /api/staff/admin/sla — list.
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
pub struct SlaListParams {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub order: Option<String>,
}

fn sla_sort_expr(sort: Option<&str>) -> &'static str {
    match sort.unwrap_or("name") {
        "status" | "isactive" => "isactive",
        "grace_period" | "grace" => "grace_period",
        "created" => "created",
        "updated" => "updated",
        _ => "name",
    }
}

/// `GET /api/staff/admin/sla` — every SLA plan (admin only). `is_default` marks
/// the plan bound to `default_sla_id`.
///
/// @implements FS-032.9: SLA plan list with sort + default flag.
pub async fn list_sla(
    State(state): State<AppState>,
    session: StaffSession,
    Query(params): Query<SlaListParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("SLA list is unavailable"))?;

    let order = match params.order.as_deref().map(|s| s.to_uppercase()) {
        Some(ref o) if o == "DESC" => "DESC",
        _ => "ASC",
    };
    let order_by = format!("{} {order}, id ASC", sla_sort_expr(params.sort.as_deref()));

    let default_id = default_sla_id(pool).await?;

    let sql = format!(
        "SELECT id, name, grace_period, isactive, enable_priority_escalation, transient, \
                disable_overdue_alerts, COALESCE(notes, '') AS notes, \
                to_char(created, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created, \
                to_char(updated, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated \
         FROM sla ORDER BY {order_by}"
    );

    // (id, name, grace, isactive, escalation, transient, disable_overdue, notes, created, updated)
    type SlaRow = (
        i32,
        String,
        i32,
        bool,
        bool,
        bool,
        bool,
        String,
        Option<String>,
        Option<String>,
    );
    let rows: Vec<SlaRow> = sqlx::query_as(&sql)
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::internal("SLA list failed"))?;

    let plans: Vec<Value> = rows
        .into_iter()
        .map(
            |(id, name, grace, isactive, escalation, transient, disable_overdue, notes, created, updated)| {
                json!({
                    "id": id,
                    "name": name,
                    "grace_period": grace,
                    "isactive": isactive,
                    "enable_priority_escalation": escalation,
                    "transient": transient,
                    "disable_overdue_alerts": disable_overdue,
                    "notes": notes,
                    "created": created,
                    "updated": updated,
                    "is_default": Some(id) == default_id,
                })
            },
        )
        .collect();

    Ok(Json(json!(plans)))
}

// ---------------------------------------------------------------------------
// POST / PUT /api/staff/admin/sla — create / edit.
// ---------------------------------------------------------------------------

/// Create/edit SLA request body.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlaWriteRequest {
    #[serde(default)]
    pub name: String,
    /// Grace period in hours — required, must be a positive integer.
    #[serde(default)]
    pub grace_period: Option<i32>,
    #[serde(default = "default_true")]
    pub isactive: bool,
    #[serde(default = "default_true")]
    pub enable_priority_escalation: bool,
    #[serde(default)]
    pub transient: bool,
    #[serde(default)]
    pub disable_overdue_alerts: bool,
    #[serde(default)]
    pub notes: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Validate the SLA name + grace period (FS-032.10). Returns the trimmed name +
/// validated grace period. KL-032.4 corrected copy (SLA-appropriate, not the
/// API-key artifact text).
fn validate_sla_fields(body: &SlaWriteRequest) -> Result<(String, i32), ApiError> {
    let mut err = ApiError::validation("Please correct the errors below");
    let name = body.name.trim().to_string();
    if name.is_empty() {
        err = err.with_field("name", "Name required");
    }
    let grace = body.grace_period.unwrap_or(0);
    if body.grace_period.is_none() || grace <= 0 {
        err = err.with_field("grace_period", "Grace period required");
    }
    if !err.fields.is_empty() {
        return Err(err);
    }
    Ok((name, grace))
}

/// Map a unique-violation on `sla_name_key` (23505) to a 422 keyed on `name`.
/// Any other DB error becomes a 500.
fn map_sla_write_error(e: sqlx::Error, context: &'static str) -> ApiError {
    if let sqlx::Error::Database(db) = &e {
        if db.code().as_deref() == Some("23505")
            && db.constraint() == Some("sla_name_key")
        {
            return ApiError::validation("An SLA plan with this name already exists")
                .with_field("name", "An SLA plan with this name already exists");
        }
    }
    tracing::warn!(error = %e, context, "sla write failed");
    ApiError::internal("Could not save the SLA plan")
}

/// `POST /api/staff/admin/sla` — create a plan (admin + CSRF).
///
/// @implements FS-032.10: create validation (name required + unique, grace > 0).
pub async fn create_sla(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    headers: http::HeaderMap,
    Json(body): Json<SlaWriteRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("SLA create is unavailable"))?;

    let (name, grace) = validate_sla_fields(&body)?;

    let id: i32 = sqlx::query_scalar(
        "INSERT INTO sla (name, grace_period, isactive, enable_priority_escalation, transient, \
                          disable_overdue_alerts, notes) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
    )
    .bind(&name)
    .bind(grace)
    .bind(body.isactive)
    .bind(body.enable_priority_escalation)
    .bind(body.transient)
    .bind(body.disable_overdue_alerts)
    .bind(body.notes.as_deref())
    .fetch_one(pool)
    .await
    .map_err(|e| map_sla_write_error(e, "create_sla"))?;

    log(
        pool,
        LogType::Debug,
        "SLA plan created",
        format!("SLA plan '{name}' (#{id}) created by staff #{}", session.staff_id),
        client_ip(&headers),
    )
    .await;

    Ok(Json(json!({ "id": id })))
}

/// `PUT /api/staff/admin/sla/:id` — edit a plan (admin + CSRF). 404 if absent.
///
/// @implements FS-032.10: edit validation (name required + unique, grace > 0).
pub async fn update_sla(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i32>,
    Json(body): Json<SlaWriteRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("SLA edit is unavailable"))?;

    let exists: Option<i32> = sqlx::query_scalar("SELECT id FROM sla WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("SLA lookup failed"))?;
    if exists.is_none() {
        return Err(ApiError::not_found("SLA plan not found"));
    }

    let (name, grace) = validate_sla_fields(&body)?;

    sqlx::query(
        "UPDATE sla SET name = $2, grace_period = $3, isactive = $4, \
                enable_priority_escalation = $5, transient = $6, \
                disable_overdue_alerts = $7, notes = $8, updated = now() \
         WHERE id = $1",
    )
    .bind(id)
    .bind(&name)
    .bind(grace)
    .bind(body.isactive)
    .bind(body.enable_priority_escalation)
    .bind(body.transient)
    .bind(body.disable_overdue_alerts)
    .bind(body.notes.as_deref())
    .execute(pool)
    .await
    .map_err(|e| map_sla_write_error(e, "update_sla"))?;

    Ok(Json(json!({ "id": id })))
}

// ---------------------------------------------------------------------------
// DELETE /api/staff/admin/sla/:id — delete + re-home dependents (FS-032.12).
// ---------------------------------------------------------------------------

/// Re-home an SLA plan's dependents then delete it, all in one transaction.
///
/// `department.sla_id` / `help_topic.sla_id` → NULL (nullable FK, NOT the legacy
/// 0 sentinel); `ticket.sla_id` → `default_sla_id`. The caller has already
/// verified `id != default_sla_id` and that a default exists.
async fn delete_sla_rehome(
    pool: &sqlx::postgres::PgPool,
    id: i32,
    default_id: i32,
) -> Result<(), ApiError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal("SLA delete failed"))?;

    sqlx::query("UPDATE department SET sla_id = NULL WHERE sla_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("SLA delete failed (department re-home)"))?;
    sqlx::query("UPDATE help_topic SET sla_id = NULL WHERE sla_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("SLA delete failed (topic re-home)"))?;
    sqlx::query("UPDATE ticket SET sla_id = $2 WHERE sla_id = $1")
        .bind(id)
        .bind(default_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("SLA delete failed (ticket re-home)"))?;
    sqlx::query("DELETE FROM sla WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("SLA delete failed"))?;

    tx.commit()
        .await
        .map_err(|_| ApiError::internal("SLA delete failed"))?;
    Ok(())
}

/// `DELETE /api/staff/admin/sla/:id` — delete a plan, re-homing dependents.
///
/// Refuses when `id == default_sla_id` or when `default_sla_id` is unset (there
/// would be no default to re-home tickets to) — FS-032.12.
///
/// @implements FS-032.12: delete re-homes dependents; default is protected.
pub async fn delete_sla(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i32>,
    headers: http::HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("SLA delete is unavailable"))?;

    let exists: Option<i32> = sqlx::query_scalar("SELECT id FROM sla WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("SLA lookup failed"))?;
    if exists.is_none() {
        return Err(ApiError::not_found("SLA plan not found"));
    }

    let default_id = default_sla_id(pool).await?;
    match default_id {
        None => {
            return Err(ApiError::validation(
                "No default SLA plan is configured, so a plan cannot be deleted (tickets have nowhere to re-home).",
            ));
        }
        Some(d) if d == id => {
            return Err(ApiError::validation(
                "The default SLA plan cannot be deleted.",
            ));
        }
        _ => {}
    }
    let default_id = default_id.expect("checked non-None above");

    delete_sla_rehome(pool, id, default_id).await?;

    log(
        pool,
        LogType::Debug,
        "SLA plan deleted",
        format!("SLA plan #{id} deleted by staff #{}", session.staff_id),
        client_ip(&headers),
    )
    .await;

    Ok(Json(json!({ "affected": 1, "message": "SLA plan deleted" })))
}

// ---------------------------------------------------------------------------
// POST /api/staff/admin/sla/mass — activate / disable / delete.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassSlaRequest {
    pub action: String,
    #[serde(default)]
    pub ids: Vec<i32>,
}

/// `POST /api/staff/admin/sla/mass` — activate / disable / delete (admin + CSRF).
///
/// Delete mirrors the single-delete protection: the default plan (and, when no
/// default is set, every plan) is skipped, dependents re-homed for the rest, and
/// a partial "N of M" message returned (like `mass_groups`).
///
/// @implements FS-032.9: SLA mass actions.
/// @implements FS-032.12: mass delete re-homes dependents; default protected.
pub async fn mass_sla(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Json(body): Json<MassSlaRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Mass action is unavailable"))?;

    let action = body.action.trim().to_lowercase();
    if !matches!(action.as_str(), "activate" | "disable" | "delete") {
        return Err(ApiError::validation("Unknown mass action"));
    }
    if body.ids.is_empty() {
        return Err(ApiError::validation("You must select at least one SLA plan."));
    }
    let ids: Vec<i32> = {
        let mut v = body.ids.clone();
        v.sort_unstable();
        v.dedup();
        v
    };
    let requested = ids.len() as i64;

    let (affected, message) = match action.as_str() {
        "activate" => {
            let n = sqlx::query("UPDATE sla SET isactive = true, updated = now() WHERE id = ANY($1)")
                .bind(&ids)
                .execute(pool)
                .await
                .map_err(|_| ApiError::internal("Activate failed"))?
                .rows_affected() as i64;
            (n, "Selected SLA plans activated".to_string())
        }
        "disable" => {
            let n = sqlx::query("UPDATE sla SET isactive = false, updated = now() WHERE id = ANY($1)")
                .bind(&ids)
                .execute(pool)
                .await
                .map_err(|_| ApiError::internal("Disable failed"))?
                .rows_affected() as i64;
            (n, "Selected SLA plans disabled".to_string())
        }
        "delete" => {
            let default_id = default_sla_id(pool).await?;
            let mut deleted = 0i64;
            // With no default configured nothing can be deleted (nowhere to
            // re-home tickets); otherwise every plan except the default one.
            if let Some(default_id) = default_id {
                for &id in &ids {
                    if id == default_id {
                        continue;
                    }
                    delete_sla_rehome(pool, id, default_id).await?;
                    deleted += 1;
                }
            }
            let msg = if deleted == requested {
                "Selected SLA plans deleted successfully".to_string()
            } else if deleted == 0 {
                "Unable to delete selected SLA plans (the default plan cannot be deleted)".to_string()
            } else {
                format!("{deleted} of {requested} selected SLA plans deleted")
            };
            (deleted, msg)
        }
        _ => unreachable!(),
    };

    Ok(Json(json!({ "affected": affected, "message": message })))
}

// ---------------------------------------------------------------------------
// GET /api/staff/admin/priorities — read-only fixed set (BS-032.13).
// ---------------------------------------------------------------------------

/// `GET /api/staff/admin/priorities` — the fixed four priorities, ranked most-
/// to-least urgent (`ORDER BY urgency DESC` — Low, Normal, High, Emergency).
/// Read-only: there is no create/edit/delete route (BS-032.13).
///
/// @implements BS-032.13: fixed priority set, read-only, ranked by urgency.
pub async fn list_priorities(
    State(state): State<AppState>,
    session: StaffSession,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Priority list is unavailable"))?;

    // (priority_id, priority, priority_desc, priority_color, urgency, ispublic)
    type PriorityRow = (i32, String, String, String, i32, bool);
    let rows: Vec<PriorityRow> = sqlx::query_as(
        "SELECT priority_id, priority, priority_desc, priority_color, urgency, ispublic \
         FROM priority ORDER BY urgency DESC, priority_id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Priority list failed"))?;

    let out: Vec<Value> = rows
        .into_iter()
        .map(|(pid, priority, desc, color, urgency, ispublic)| {
            json!({
                "priority_id": pid,
                "priority": priority,
                "priority_desc": desc,
                "priority_color": color,
                "urgency": urgency,
                "ispublic": ispublic,
            })
        })
        .collect();

    Ok(Json(json!(out)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_expr_maps_keys() {
        assert_eq!(sla_sort_expr(None), "name");
        assert_eq!(sla_sort_expr(Some("created")), "created");
        assert_eq!(sla_sort_expr(Some("grace")), "grace_period");
        assert_eq!(sla_sort_expr(Some("status")), "isactive");
    }

    #[test]
    fn validate_rejects_missing_name_and_grace() {
        let body = SlaWriteRequest::default();
        let err = validate_sla_fields(&body).unwrap_err();
        assert!(err.fields.contains_key("name"));
        assert!(err.fields.contains_key("grace_period"));
    }

    #[test]
    fn validate_rejects_nonpositive_grace() {
        let body = SlaWriteRequest {
            name: "Gold".into(),
            grace_period: Some(0),
            ..Default::default()
        };
        let err = validate_sla_fields(&body).unwrap_err();
        assert!(err.fields.contains_key("grace_period"));
        assert!(!err.fields.contains_key("name"));
    }

    #[test]
    fn validate_accepts_valid() {
        let body = SlaWriteRequest {
            name: "  Gold  ".into(),
            grace_period: Some(24),
            ..Default::default()
        };
        let (name, grace) = validate_sla_fields(&body).unwrap();
        assert_eq!(name, "Gold");
        assert_eq!(grace, 24);
    }
}
