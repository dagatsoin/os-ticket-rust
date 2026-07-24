//! Admin permission-group management endpoints (TS-M4-B3) — FS-031.7/.8.
//!
//! Admin-gated CRUD over the `groups` table plus the department-access matrix
//! (`group_dept_access`). Carries the canonical eleven-flag permission set
//! (BS-031-020) including the four net-new M4 capability flags, which flow into
//! `GET /api/staff/me` for session-bootstrap gating.
//!
//! @implements FS-031.7: group list (member/dept counts, sort, mass actions).
//! @implements FS-031.8: create / edit a group.
//! @implements BS-031-019: unique name, ≥3 chars.
//! @implements BS-031-020: canonical eleven permission flags.
//! @implements BS-031-021: department-access full-replace reconcile.
//! @implements BS-031-022: group with members cannot be deleted.
//! @implements BS-031-023: self-group protection on mass actions.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use ost_core::ApiError;

use crate::auth::gate::require_admin;
use crate::auth::realm::StaffSession;
use crate::auth::StaffCsrf;
use crate::state::AppState;

/// The canonical eleven permission flags (BS-031-020), in storage-key form. Both
/// the create/edit persistence and the detail read iterate this list, so the set
/// can never drift between write and read.
const FLAG_KEYS: [&str; 11] = [
    "can_create_tickets",
    "can_edit_tickets",
    "can_post_reply",
    "can_close_tickets",
    "can_assign_tickets",
    "can_transfer_tickets",
    "can_delete_tickets",
    "can_manage_faq",
    "can_manage_premade",
    "can_ban_emails",
    "can_view_staff_stats",
];

// ---------------------------------------------------------------------------
// GET /api/staff/admin/groups — list.
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
pub struct GroupListParams {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub order: Option<String>,
}

fn group_sort_expr(sort: Option<&str>) -> &'static str {
    match sort.unwrap_or("name") {
        "status" => "g.group_enabled",
        "users" => "member_count",
        "depts" => "dept_count",
        "created" => "g.created",
        "updated" => "g.updated",
        _ => "g.group_name",
    }
}

/// `GET /api/staff/admin/groups` — every group with member + dept counts (admin
/// only). Not paginated; left-join counts so zero-member groups still appear
/// (BS-031-034).
///
/// @implements FS-031.7: group list with counts + sort.
pub async fn list_groups(
    State(state): State<AppState>,
    session: StaffSession,
    Query(params): Query<GroupListParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Group list is unavailable"))?;

    let order = match params.order.as_deref().map(|s| s.to_uppercase()) {
        Some(ref o) if o == "DESC" => "DESC",
        _ => "ASC",
    };
    let order_by = format!("{} {order}, g.group_id ASC", group_sort_expr(params.sort.as_deref()));

    let sql = format!(
        "SELECT g.group_id, g.group_name, g.group_enabled, \
                COALESCE(m.member_count, 0) AS member_count, \
                COALESCE(da.dept_count, 0) AS dept_count, \
                to_char(g.created, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created, \
                to_char(g.updated, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated \
         FROM groups g \
         LEFT JOIN (SELECT group_id, COUNT(*) AS member_count FROM staff GROUP BY group_id) m \
                ON m.group_id = g.group_id \
         LEFT JOIN (SELECT group_id, COUNT(*) AS dept_count FROM group_dept_access GROUP BY group_id) da \
                ON da.group_id = g.group_id \
         ORDER BY {order_by}"
    );

    // (id, name, enabled, member_count, dept_count, created, updated)
    type GroupListRow = (i32, String, bool, i64, i64, Option<String>, Option<String>);
    let rows: Vec<GroupListRow> = sqlx::query_as(&sql)
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::internal("Group list failed"))?;

    let groups: Vec<Value> = rows
        .into_iter()
        .map(|(id, name, enabled, members, depts, created, updated)| {
            json!({
                "id": id,
                "name": name,
                "enabled": enabled,
                "member_count": members,
                "dept_count": depts,
                "created": created,
                "updated": updated,
            })
        })
        .collect();

    Ok(Json(json!(groups)))
}

// ---------------------------------------------------------------------------
// GET /api/staff/admin/groups/:id — detail.
// ---------------------------------------------------------------------------

/// `GET /api/staff/admin/groups/:id` — a single group with its flags + dept set.
///
/// @implements FS-031.8: group detail (flags + department access).
pub async fn get_group(
    State(state): State<AppState>,
    session: StaffSession,
    Path(id): Path<i32>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Group lookup is unavailable"))?;

    let sql = format!(
        "SELECT group_id, group_name, group_enabled, COALESCE(notes, '') AS notes, {} \
         FROM groups WHERE group_id = $1",
        FLAG_KEYS.join(", ")
    );
    // (id, name, enabled, notes, + the eleven FLAG_KEYS booleans in order).
    type GroupDetailRow = (
        i32, String, bool, String, bool, bool, bool, bool, bool, bool, bool, bool, bool, bool, bool,
    );
    let row: Option<GroupDetailRow> = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::internal("Group lookup failed"))?;
    let row = row.ok_or_else(|| ApiError::not_found("Group not found"))?;

    let flag_vals = [
        row.4, row.5, row.6, row.7, row.8, row.9, row.10, row.11, row.12, row.13, row.14,
    ];
    let mut flags = serde_json::Map::new();
    for (k, v) in FLAG_KEYS.iter().zip(flag_vals.iter()) {
        flags.insert((*k).to_string(), json!(*v));
    }

    let dept_ids: Vec<i32> =
        sqlx::query_scalar("SELECT dept_id FROM group_dept_access WHERE group_id = $1 ORDER BY dept_id")
            .bind(id)
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::internal("Dept-access lookup failed"))?;

    Ok(Json(json!({
        "id": row.0,
        "name": row.1,
        "enabled": row.2,
        "notes": row.3,
        "flags": Value::Object(flags),
        "dept_ids": dept_ids,
    })))
}

// ---------------------------------------------------------------------------
// POST / PUT /api/staff/admin/groups — create / edit.
// ---------------------------------------------------------------------------

/// Create/edit group request body.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupWriteRequest {
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// The canonical eleven flags by storage key (missing → false).
    #[serde(default)]
    pub flags: std::collections::HashMap<String, bool>,
    #[serde(default)]
    pub dept_ids: Vec<i32>,
    #[serde(default)]
    pub notes: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Validate the group name (BS-031-019). Returns the trimmed name on success.
async fn validate_group_name(
    pool: &sqlx::postgres::PgPool,
    name: &str,
    exclude_id: Option<i32>,
) -> Result<String, ApiError> {
    let n = name.trim();
    if n.is_empty() {
        return Err(ApiError::validation("Group name required").with_field("name", "Group name required"));
    }
    if n.chars().count() < 3 {
        return Err(ApiError::validation("Group name must be at least 3 chars.")
            .with_field("name", "Group name must be at least 3 chars."));
    }
    let clash: Option<i32> = match exclude_id {
        Some(id) => {
            sqlx::query_scalar("SELECT group_id FROM groups WHERE lower(group_name) = lower($1) AND group_id <> $2")
                .bind(n)
                .bind(id)
                .fetch_optional(pool)
                .await
        }
        None => {
            sqlx::query_scalar("SELECT group_id FROM groups WHERE lower(group_name) = lower($1)")
                .bind(n)
                .fetch_optional(pool)
                .await
        }
    }
    .map_err(|_| ApiError::internal("Group lookup failed"))?;
    if clash.is_some() {
        return Err(ApiError::validation("Group name already exists")
            .with_field("name", "Group name already exists"));
    }
    Ok(n.to_string())
}

/// Resolve the eleven flag values from the request map (missing → false).
fn resolve_flags(flags: &std::collections::HashMap<String, bool>) -> [bool; 11] {
    let mut out = [false; 11];
    for (i, k) in FLAG_KEYS.iter().enumerate() {
        out[i] = flags.get(*k).copied().unwrap_or(false);
    }
    out
}

/// Replace a group's department-access set (BS-031-021: delete-then-insert).
async fn replace_dept_access(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    group_id: i32,
    dept_ids: &[i32],
) -> Result<(), ApiError> {
    sqlx::query("DELETE FROM group_dept_access WHERE group_id = $1")
        .bind(group_id)
        .execute(&mut **tx)
        .await
        .map_err(|_| ApiError::internal("Dept-access reset failed"))?;
    let mut seen = std::collections::HashSet::new();
    for &dept_id in dept_ids {
        if !seen.insert(dept_id) {
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
            tracing::warn!(error = %e, "replace_dept_access: insert failed");
            ApiError::internal("Dept-access update failed")
        })?;
    }
    Ok(())
}

/// `POST /api/staff/admin/groups` — create a group (admin + CSRF).
///
/// @implements FS-031.8: create with name validation, eleven flags, dept access.
pub async fn create_group(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    headers: http::HeaderMap,
    Json(body): Json<GroupWriteRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Group create is unavailable"))?;

    let name = validate_group_name(pool, &body.name, None).await?;
    let flags = resolve_flags(&body.flags);

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal("Group create failed"))?;

    // Columns: group_name($1), group_enabled($2), notes($3), then the 11 flags.
    let cols = FLAG_KEYS.join(", ");
    let placeholders = (0..11).map(|i| format!("${}", i + 4)).collect::<Vec<_>>().join(", ");
    let sql = format!(
        "INSERT INTO groups (group_name, group_enabled, notes, {cols}) \
         VALUES ($1, $2, $3, {placeholders}) RETURNING group_id"
    );

    let mut q = sqlx::query_scalar::<_, i32>(&sql)
        .bind(&name)
        .bind(body.enabled)
        .bind(body.notes.as_deref());
    for f in flags {
        q = q.bind(f);
    }
    let group_id: i32 = q
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "create_group: insert failed");
            ApiError::internal("Could not create group")
        })?;

    replace_dept_access(&mut tx, group_id, &body.dept_ids).await?;

    tx.commit()
        .await
        .map_err(|_| ApiError::internal("Group create failed"))?;

    // TS-M4-G1: audit the admin CRUD create (Debug, best-effort).
    ost_core::log(
        pool,
        ost_core::LogType::Debug,
        "Group created",
        format!("Group '{name}' (#{group_id}) created by staff #{}", session.staff_id),
        crate::logs::client_ip(&headers),
    )
    .await;

    Ok(Json(json!({ "id": group_id })))
}

/// `PUT /api/staff/admin/groups/:id` — edit a group (admin + CSRF).
///
/// @implements FS-031.8: edit with name validation, eleven flags, dept reconcile.
/// @implements BS-031-021: department-access full-replace.
pub async fn update_group(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i32>,
    Json(body): Json<GroupWriteRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Group edit is unavailable"))?;

    // Must exist.
    let exists: Option<i32> = sqlx::query_scalar("SELECT group_id FROM groups WHERE group_id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("Group lookup failed"))?;
    if exists.is_none() {
        return Err(ApiError::not_found("Group not found"));
    }

    let name = validate_group_name(pool, &body.name, Some(id)).await?;
    let flags = resolve_flags(&body.flags);

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal("Group edit failed"))?;

    let set_flags = FLAG_KEYS
        .iter()
        .enumerate()
        .map(|(i, k)| format!("{k} = ${}", i + 5))
        .collect::<Vec<_>>()
        .join(", ");
    // $1 = group_id, $2 = name, $3 = enabled, $4 = notes, $5.. = flags.
    let sql = format!(
        "UPDATE groups SET group_name = $2, group_enabled = $3, notes = $4, {set_flags}, \
                updated = now() \
         WHERE group_id = $1"
    );
    let mut q = sqlx::query(&sql)
        .bind(id)
        .bind(&name)
        .bind(body.enabled)
        .bind(body.notes.as_deref());
    for f in flags {
        q = q.bind(f);
    }
    q.execute(&mut *tx).await.map_err(|e| {
        tracing::warn!(error = %e, "update_group: update failed");
        ApiError::internal("Could not update group")
    })?;

    replace_dept_access(&mut tx, id, &body.dept_ids).await?;

    tx.commit()
        .await
        .map_err(|_| ApiError::internal("Group edit failed"))?;

    Ok(Json(json!({ "id": id })))
}

// ---------------------------------------------------------------------------
// POST /api/staff/admin/groups/mass — enable / disable / delete.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassGroupRequest {
    pub action: String,
    #[serde(default)]
    pub ids: Vec<i32>,
}

/// `POST /api/staff/admin/groups/mass` — enable / disable / delete (admin + CSRF).
///
/// @implements FS-031.7: group mass actions.
/// @implements BS-031-022: delete only zero-member groups (partial message).
/// @implements BS-031-023: self-group protection on disable / delete.
pub async fn mass_groups(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    headers: http::HeaderMap,
    Json(body): Json<MassGroupRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Mass action is unavailable"))?;

    let action = body.action.trim().to_lowercase();
    if !matches!(action.as_str(), "enable" | "disable" | "delete") {
        return Err(ApiError::validation("Unknown mass action"));
    }
    if body.ids.is_empty() {
        return Err(ApiError::validation("You must select at least one group."));
    }
    let ids: Vec<i32> = {
        let mut v = body.ids.clone();
        v.sort_unstable();
        v.dedup();
        v
    };

    // BS-031-023: the acting admin may not disable/delete a group they belong to.
    if matches!(action.as_str(), "disable" | "delete") {
        let my_group: Option<i32> =
            sqlx::query_scalar("SELECT group_id FROM staff WHERE staff_id = $1")
                .bind(session.staff_id)
                .fetch_optional(pool)
                .await
                .map_err(|_| ApiError::internal("Group lookup failed"))?;
        if let Some(g) = my_group {
            if ids.contains(&g) {
                return Err(ApiError::validation(
                    "As an admin, you can't disable/delete a group you belong to - you might lockout all admins!",
                ));
            }
        }
    }

    let requested = ids.len() as i64;
    let (affected, message) = match action.as_str() {
        "enable" => {
            let n = sqlx::query("UPDATE groups SET group_enabled = true, updated = now() WHERE group_id = ANY($1)")
                .bind(&ids)
                .execute(pool)
                .await
                .map_err(|_| ApiError::internal("Enable failed"))?
                .rows_affected() as i64;
            (n, "Selected groups activated".to_string())
        }
        "disable" => {
            let n = sqlx::query("UPDATE groups SET group_enabled = false, updated = now() WHERE group_id = ANY($1)")
                .bind(&ids)
                .execute(pool)
                .await
                .map_err(|_| ApiError::internal("Disable failed"))?
                .rows_affected() as i64;
            (n, "Selected groups disabled".to_string())
        }
        "delete" => {
            // BS-031-022: only groups with zero members are deletable.
            let deletable: Vec<i32> = sqlx::query_scalar(
                "SELECT g.group_id FROM groups g \
                 WHERE g.group_id = ANY($1) \
                   AND NOT EXISTS (SELECT 1 FROM staff s WHERE s.group_id = g.group_id)",
            )
            .bind(&ids)
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::internal("Group lookup failed"))?;

            let n = if deletable.is_empty() {
                0
            } else {
                // group_dept_access is ON DELETE CASCADE, so the access rows go too.
                sqlx::query("DELETE FROM groups WHERE group_id = ANY($1)")
                    .bind(&deletable)
                    .execute(pool)
                    .await
                    .map_err(|_| ApiError::internal("Delete failed"))?
                    .rows_affected() as i64
            };
            let msg = if n == requested {
                "Selected groups deleted successfully".to_string()
            } else if n == 0 {
                "Unable to delete selected groups".to_string()
            } else {
                format!("{n} of {requested} selected groups deleted")
            };
            (n, msg)
        }
        _ => unreachable!(),
    };

    // TS-M4-G1: audit an admin CRUD delete (Debug, best-effort).
    if action == "delete" && affected > 0 {
        ost_core::log(
            pool,
            ost_core::LogType::Debug,
            "Groups deleted",
            format!("{affected} group(s) deleted by staff #{}", session.staff_id),
            crate::logs::client_ip(&headers),
        )
        .await;
    }

    Ok(Json(json!({ "affected": affected, "message": message })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_set_is_the_canonical_eleven() {
        assert_eq!(FLAG_KEYS.len(), 11);
        // The four net-new M4 flags are present.
        for k in ["can_manage_faq", "can_manage_premade", "can_ban_emails", "can_view_staff_stats"] {
            assert!(FLAG_KEYS.contains(&k), "missing net-new flag {k}");
        }
    }

    #[test]
    fn resolve_flags_defaults_missing_to_false() {
        let mut m = std::collections::HashMap::new();
        m.insert("can_manage_faq".to_string(), true);
        m.insert("can_create_tickets".to_string(), true);
        let out = resolve_flags(&m);
        // can_create_tickets is index 0; can_manage_faq is index 7.
        assert!(out[0]);
        assert!(out[7]);
        // can_post_reply (index 2) not supplied → false.
        assert!(!out[2]);
    }

    #[test]
    fn group_sort_expr_maps_keys() {
        assert_eq!(group_sort_expr(None), "g.group_name");
        assert_eq!(group_sort_expr(Some("users")), "member_count");
        assert_eq!(group_sort_expr(Some("depts")), "dept_count");
    }
}
