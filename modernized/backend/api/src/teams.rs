//! Admin team management (TS-M4-C3) — FS-030.10/.11/.12.
//!
//! Admin-gated CRUD over the `team` table. The form removes members only (adding
//! a member is owned by TS-M4-B1's `POST /api/staff/admin/staff/:id/teams`); the
//! lead must be a current member; delete releases ticket + help-topic associations
//! (the latter is a RESTRICT FK, so it MUST be cleared before the row is removed).
//!
//! Route shapes (all under `/api/staff/admin`, admin-gated per-handler; mutating
//! routes CSRF-enforced):
//! * `GET  /teams` — list (lead name + member count).
//! * `GET  /teams/:id` — one team + its members.
//! * `POST /teams` — create.
//! * `PUT  /teams/:id` — edit (apply member removals, re-validate lead).
//! * `POST /teams/mass` — enable / disable / delete.
//! * `DELETE /teams/:id` — delete + release ticket/topic associations.
//!
//! @implements FS-030.10: team list + create.
//! @implements FS-030.11: team edit (lead-from-members, member removal only).
//! @implements FS-030.12: team delete releases ticket + help-topic associations.
//! @implements BS-030-12: name required, ≥3 chars, unique.
//! @implements BS-030-13: lead must be a current member; removing it resets to NULL.
//! @implements BS-030-14: the team form removes members only (no add).
//! @implements BS-030-16: delete clears ticket.team_id=0 + help_topic.team_id=NULL.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use ost_core::ApiError;

use crate::auth::gate::require_admin;
use crate::auth::realm::StaffSession;
use crate::auth::StaffCsrf;
use crate::state::AppState;

fn pool(state: &AppState) -> Result<&sqlx::postgres::PgPool, ApiError> {
    state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Teams are unavailable"))
}

// ---------------------------------------------------------------------------
// GET /api/staff/admin/teams — list.
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
pub struct TeamListParams {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub order: Option<String>,
}

fn team_sort_expr(sort: Option<&str>) -> &'static str {
    match sort.unwrap_or("name") {
        "status" | "isenabled" => "t.isenabled",
        "members" => "member_count",
        "updated" => "t.updated",
        "created" => "t.created",
        _ => "t.name",
    }
}

/// `GET /api/staff/admin/teams` — every team (admin only).
///
/// @implements FS-030.10: team list with lead name + member count.
pub async fn list_teams(
    State(state): State<AppState>,
    session: StaffSession,
    Query(params): Query<TeamListParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    let order = match params.order.as_deref().map(|s| s.to_uppercase()) {
        Some(ref o) if o == "DESC" => "DESC",
        _ => "ASC",
    };
    let order_by = format!("{} {order}, t.team_id ASC", team_sort_expr(params.sort.as_deref()));

    let sql = format!(
        "SELECT t.team_id, t.name, t.isenabled, t.lead_id, \
                COALESCE(NULLIF(TRIM(l.firstname || ' ' || l.lastname), ''), l.username, '') AS lead_name, \
                COALESCE(mc.member_count, 0) AS member_count, \
                to_char(t.updated, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated \
         FROM team t \
         LEFT JOIN staff l ON l.staff_id = t.lead_id \
         LEFT JOIN (SELECT team_id, COUNT(*) AS member_count FROM team_member GROUP BY team_id) mc \
                ON mc.team_id = t.team_id \
         ORDER BY {order_by}"
    );

    // (team_id, name, isenabled, lead_id, lead_name, member_count, updated)
    type Row = (i32, String, bool, Option<i32>, String, i64, Option<String>);
    let rows: Vec<Row> = sqlx::query_as(&sql)
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::internal("Team list failed"))?;

    let out: Vec<Value> = rows
        .into_iter()
        .map(|(id, name, isenabled, lead_id, lead_name, member_count, updated)| {
            json!({
                "id": id,
                "name": name,
                "isenabled": isenabled,
                "lead_id": lead_id,
                "lead_name": lead_name,
                "member_count": member_count,
                "updated": updated,
            })
        })
        .collect();

    Ok(Json(json!(out)))
}

// ---------------------------------------------------------------------------
// GET /api/staff/admin/teams/:id — detail.
// ---------------------------------------------------------------------------

/// `GET /api/staff/admin/teams/:id` — one team + its members.
///
/// @implements FS-030.11: team detail (row + member list).
pub async fn get_team(
    State(state): State<AppState>,
    session: StaffSession,
    Path(id): Path<i32>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    // (team_id, name, isenabled, lead_id, noalerts, notes, updated)
    type Row = (i32, String, bool, Option<i32>, bool, Option<String>, Option<String>);
    let row: Option<Row> = sqlx::query_as(
        "SELECT team_id, name, isenabled, lead_id, noalerts, notes, \
                to_char(updated, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated \
         FROM team WHERE team_id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal("Team lookup failed"))?;

    let row = row.ok_or_else(|| ApiError::not_found("Team not found"))?;

    // (staff_id, name)
    type MemberRow = (i32, String);
    let member_rows: Vec<MemberRow> = sqlx::query_as(
        "SELECT s.staff_id, COALESCE(NULLIF(TRIM(s.firstname || ' ' || s.lastname), ''), s.username) AS name \
         FROM team_member tm JOIN staff s ON s.staff_id = tm.staff_id \
         WHERE tm.team_id = $1 ORDER BY name ASC",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Team member lookup failed"))?;
    let members: Vec<Value> = member_rows
        .into_iter()
        .map(|(staff_id, name)| json!({ "staff_id": staff_id, "name": name }))
        .collect();

    Ok(Json(json!({
        "id": row.0,
        "name": row.1,
        "isenabled": row.2,
        "lead_id": row.3,
        "noalerts": row.4,
        "notes": row.5.unwrap_or_default(),
        "updated": row.6,
        "members": members,
    })))
}

// ---------------------------------------------------------------------------
// POST / PUT — create / edit.
// ---------------------------------------------------------------------------

/// Create/edit team request body (camelCase).
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamWriteRequest {
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_true")]
    pub isenabled: bool,
    #[serde(default)]
    pub lead_id: Option<i32>,
    #[serde(default)]
    pub noalerts: bool,
    #[serde(default)]
    pub notes: Option<String>,
    /// Members to remove (BS-030-14: the form only removes; add is staff-side).
    #[serde(default)]
    pub remove_member_ids: Vec<i32>,
}

fn default_true() -> bool {
    true
}

/// Validate the team name (BS-030-12). Returns the trimmed name.
fn validate_team_name(body: &TeamWriteRequest) -> Result<String, ApiError> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::validation("Team name required").with_field("name", "Team name required"));
    }
    if name.chars().count() < 3 {
        return Err(ApiError::validation("Team name must be at least 3 chars.")
            .with_field("name", "Team name must be at least 3 chars."));
    }
    Ok(name)
}

/// Map a unique-violation on `team_name_key` (23505) to a 422 keyed on `name`.
fn map_team_write_error(e: sqlx::Error, context: &'static str) -> ApiError {
    if let sqlx::Error::Database(db) = &e {
        if db.code().as_deref() == Some("23505") && db.constraint() == Some("team_name_key") {
            return ApiError::validation("Team name already exists")
                .with_field("name", "Team name already exists");
        }
    }
    tracing::warn!(error = %e, context, "team write failed");
    ApiError::internal("Could not save the team")
}

/// Resolve the final `lead_id` (BS-030-13). `current_members` is the membership
/// AFTER removals were applied. A supplied lead must be among them; otherwise a
/// supplied-but-absent lead is a 422. When no lead is supplied (or it's the 0
/// sentinel), the existing lead is kept only if it's still a member.
fn resolve_lead(
    supplied: Option<i32>,
    existing: Option<i32>,
    current_members: &[i32],
) -> Result<Option<i32>, ApiError> {
    match supplied.filter(|&v| v > 0) {
        Some(lead) => {
            if current_members.contains(&lead) {
                Ok(Some(lead))
            } else {
                Err(ApiError::validation("Team lead must be a team member")
                    .with_field("leadId", "Team lead must be a team member"))
            }
        }
        None => match existing {
            Some(prev) if current_members.contains(&prev) => Ok(Some(prev)),
            _ => Ok(None),
        },
    }
}

/// `POST /api/staff/admin/teams` — create (admin + CSRF). A new team has no
/// members, so a supplied `leadId` is rejected (BS-030-13).
///
/// @implements FS-030.10: team create.
pub async fn create_team(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Json(body): Json<TeamWriteRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    let name = validate_team_name(&body)?;
    // A new team has no members yet → a lead cannot be a current member.
    let lead = resolve_lead(body.lead_id, None, &[])?;

    let id: i32 = sqlx::query_scalar(
        "INSERT INTO team (name, isenabled, lead_id, noalerts, notes) \
         VALUES ($1, $2, $3, $4, $5) RETURNING team_id",
    )
    .bind(&name)
    .bind(body.isenabled)
    .bind(lead)
    .bind(body.noalerts)
    .bind(body.notes.as_deref())
    .fetch_one(pool)
    .await
    .map_err(|e| map_team_write_error(e, "create_team"))?;

    Ok(Json(json!({ "id": id })))
}

/// `PUT /api/staff/admin/teams/:id` — edit (admin + CSRF). Applies member removals
/// first, then re-validates the lead against the remaining membership.
///
/// @implements FS-030.11: team edit (member removal + lead re-validation).
/// @implements BS-030-13: lead must be a current member; removing it resets to NULL.
pub async fn update_team(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i32>,
    Json(body): Json<TeamWriteRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    let existing_lead: Option<Option<i32>> =
        sqlx::query_scalar("SELECT lead_id FROM team WHERE team_id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::internal("Team lookup failed"))?;
    let Some(existing_lead) = existing_lead else {
        return Err(ApiError::not_found("Team not found"));
    };

    let name = validate_team_name(&body)?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal("Team edit failed"))?;

    // BS-030-14: apply member removals (the form only removes members).
    if !body.remove_member_ids.is_empty() {
        sqlx::query("DELETE FROM team_member WHERE team_id = $1 AND staff_id = ANY($2)")
            .bind(id)
            .bind(&body.remove_member_ids)
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::internal("Member removal failed"))?;
    }

    // Membership AFTER removals — the set the lead must belong to.
    let current_members: Vec<i32> =
        sqlx::query_scalar("SELECT staff_id FROM team_member WHERE team_id = $1")
            .bind(id)
            .fetch_all(&mut *tx)
            .await
            .map_err(|_| ApiError::internal("Member lookup failed"))?;

    let lead = resolve_lead(body.lead_id, existing_lead, &current_members)?;

    sqlx::query(
        "UPDATE team SET name = $2, isenabled = $3, lead_id = $4, noalerts = $5, notes = $6, \
                updated = now() WHERE team_id = $1",
    )
    .bind(id)
    .bind(&name)
    .bind(body.isenabled)
    .bind(lead)
    .bind(body.noalerts)
    .bind(body.notes.as_deref())
    .execute(&mut *tx)
    .await
    .map_err(|e| map_team_write_error(e, "update_team"))?;

    tx.commit()
        .await
        .map_err(|_| ApiError::internal("Team edit failed"))?;

    Ok(Json(json!({ "id": id })))
}

// ---------------------------------------------------------------------------
// DELETE /api/staff/admin/teams/:id — delete + release associations.
// ---------------------------------------------------------------------------

/// Release a team's associations then delete it, in one transaction. `ticket.team_id`
/// → 0 (loose sentinel); `help_topic.team_id` → NULL (RESTRICT FK — MUST be cleared
/// first); `team_member` rows cascade on delete.
async fn delete_team_release(pool: &sqlx::postgres::PgPool, id: i32) -> Result<(), ApiError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal("Team delete failed"))?;

    sqlx::query("UPDATE ticket SET team_id = 0, updated = now() WHERE team_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("Team delete failed (ticket release)"))?;
    sqlx::query("UPDATE help_topic SET team_id = NULL, updated = now() WHERE team_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("Team delete failed (topic release)"))?;
    // team_member is ON DELETE CASCADE — the rows go with the team.
    sqlx::query("DELETE FROM team WHERE team_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("Team delete failed"))?;

    tx.commit()
        .await
        .map_err(|_| ApiError::internal("Team delete failed"))?;
    Ok(())
}

/// `DELETE /api/staff/admin/teams/:id` — delete + release associations.
///
/// @implements FS-030.12: team delete releases ticket + help-topic associations.
/// @implements BS-030-16: ticket.team_id=0 + help_topic.team_id=NULL then delete.
pub async fn delete_team(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i32>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    let exists: Option<i32> = sqlx::query_scalar("SELECT team_id FROM team WHERE team_id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("Team lookup failed"))?;
    if exists.is_none() {
        return Err(ApiError::not_found("Team not found"));
    }

    delete_team_release(pool, id).await?;

    Ok(Json(json!({ "affected": 1, "message": "Team deleted" })))
}

// ---------------------------------------------------------------------------
// POST /api/staff/admin/teams/mass — enable / disable / delete.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassTeamRequest {
    pub action: String,
    #[serde(default)]
    pub ids: Vec<i32>,
}

/// `POST /api/staff/admin/teams/mass` — enable / disable / delete (admin + CSRF).
///
/// @implements FS-030.10: team mass actions.
/// @implements BS-030-16: mass delete releases each team's associations.
pub async fn mass_teams(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Json(body): Json<MassTeamRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    let action = body.action.trim().to_lowercase();
    if !matches!(action.as_str(), "enable" | "disable" | "delete") {
        return Err(ApiError::validation("Unknown mass action"));
    }
    if body.ids.is_empty() {
        return Err(ApiError::validation("You must select at least one team."));
    }
    let ids: Vec<i32> = {
        let mut v = body.ids.clone();
        v.sort_unstable();
        v.dedup();
        v
    };

    let (affected, message) = match action.as_str() {
        "enable" => {
            let n = sqlx::query("UPDATE team SET isenabled = true, updated = now() WHERE team_id = ANY($1)")
                .bind(&ids)
                .execute(pool)
                .await
                .map_err(|_| ApiError::internal("Enable failed"))?
                .rows_affected() as i64;
            (n, "Selected teams enabled".to_string())
        }
        "disable" => {
            let n = sqlx::query("UPDATE team SET isenabled = false, updated = now() WHERE team_id = ANY($1)")
                .bind(&ids)
                .execute(pool)
                .await
                .map_err(|_| ApiError::internal("Disable failed"))?
                .rows_affected() as i64;
            (n, "Selected teams disabled".to_string())
        }
        "delete" => {
            let mut deleted = 0i64;
            for &id in &ids {
                delete_team_release(pool, id).await?;
                deleted += 1;
            }
            (deleted, "Selected teams deleted".to_string())
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
        assert_eq!(team_sort_expr(None), "t.name");
        assert_eq!(team_sort_expr(Some("members")), "member_count");
        assert_eq!(team_sort_expr(Some("updated")), "t.updated");
        assert_eq!(team_sort_expr(Some("status")), "t.isenabled");
    }

    #[test]
    fn name_validation() {
        let short = TeamWriteRequest { name: "AB".into(), ..Default::default() };
        assert!(validate_team_name(&short).unwrap_err().fields.contains_key("name"));
        let ok = TeamWriteRequest { name: "  Tier 3  ".into(), ..Default::default() };
        assert_eq!(validate_team_name(&ok).unwrap(), "Tier 3");
    }

    #[test]
    fn lead_must_be_a_current_member() {
        // Supplied non-member → error.
        assert!(resolve_lead(Some(7), None, &[1, 2]).is_err());
        // Supplied member → kept.
        assert_eq!(resolve_lead(Some(2), None, &[1, 2]).unwrap(), Some(2));
        // No lead supplied, existing still a member → kept.
        assert_eq!(resolve_lead(None, Some(1), &[1, 2]).unwrap(), Some(1));
        // No lead supplied, existing removed → reset to NULL.
        assert_eq!(resolve_lead(None, Some(3), &[1, 2]).unwrap(), None);
        // Zero sentinel behaves like "not supplied".
        assert_eq!(resolve_lead(Some(0), Some(1), &[1]).unwrap(), Some(1));
    }
}
