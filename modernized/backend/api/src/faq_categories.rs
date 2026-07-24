//! FAQ-category management (TS-M4-E1) — FS-032.13/.14/.16.
//!
//! CRUD over the `faq_category` table, gated by the **delegated** capability
//! `can_manage_faq` (NOT the blanket admin gate): an administrator always passes,
//! and a non-admin staff member passes only when their group carries the flag.
//! This is the first delegated-capability surface in M4, so the routes live under
//! `/api/staff/faq-categories` (NOT `/admin`).
//!
//! Route shapes (every handler gated by `require_admin_or_permission(can_manage_faq)`;
//! mutating routes additionally CSRF-enforced):
//! * `GET  /faq-categories` — list every category.
//! * `GET  /faq-categories/:id` — one category (404 when absent, KL-032.11).
//! * `POST /faq-categories` — create (name required).
//! * `PUT  /faq-categories/:id` — edit (404 when absent).
//! * `POST /faq-categories/mass` — delete / makepublic / makeprivate.
//! * `DELETE /faq-categories/:id` — plain delete (cascade inert until M7 articles).
//!
//! @implements FS-032.13: FAQ category list + detail.
//! @implements FS-032.14: create/edit validation (name required).
//! @implements FS-032.15: `can_manage_faq` delegated gate (admin OR flag).
//! @implements FS-032.16: plain delete (article cascade defined-but-inert).
//! @implements KL-032.11: missing id returns 404, not a hollow object.

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use ost_core::ApiError;

use crate::auth::gate::require_admin_or_permission;
use crate::auth::realm::StaffSession;
use crate::auth::StaffCsrf;
use crate::state::AppState;

/// The delegated capability that guards every FAQ-category route.
const CAP: &str = "can_manage_faq";

fn pool(state: &AppState) -> Result<&sqlx::postgres::PgPool, ApiError> {
    state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("FAQ categories are unavailable"))
}

// ---------------------------------------------------------------------------
// GET /api/staff/faq-categories — list.
// ---------------------------------------------------------------------------

/// `GET /api/staff/faq-categories` — every category (admin OR `can_manage_faq`).
///
/// @implements FS-032.13: FAQ category list.
/// @implements FS-032.15: `can_manage_faq` delegated gate.
pub async fn list_categories(
    State(state): State<AppState>,
    session: StaffSession,
) -> Result<Json<Value>, ApiError> {
    require_admin_or_permission(&state, &session, CAP).await?;
    let pool = pool(&state)?;

    // (id, name, ispublic, description, notes, updated)
    type Row = (i32, String, bool, String, Option<String>, Option<String>);
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, name, ispublic, description, notes, \
                to_char(updated, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated \
         FROM faq_category ORDER BY name ASC, id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("FAQ category list failed"))?;

    let out: Vec<Value> = rows
        .into_iter()
        .map(|(id, name, ispublic, description, notes, updated)| {
            json!({
                "id": id,
                "name": name,
                "ispublic": ispublic,
                "description": description,
                "notes": notes.unwrap_or_default(),
                "updated": updated,
            })
        })
        .collect();

    Ok(Json(json!(out)))
}

// ---------------------------------------------------------------------------
// GET /api/staff/faq-categories/:id — detail.
// ---------------------------------------------------------------------------

/// `GET /api/staff/faq-categories/:id` — one category; 404 when absent.
///
/// @implements FS-032.13: FAQ category detail.
/// @implements KL-032.11: missing id returns 404, not a hollow object.
pub async fn get_category(
    State(state): State<AppState>,
    session: StaffSession,
    Path(id): Path<i32>,
) -> Result<Json<Value>, ApiError> {
    require_admin_or_permission(&state, &session, CAP).await?;
    let pool = pool(&state)?;

    // (id, name, ispublic, description, notes, updated)
    type Row = (i32, String, bool, String, Option<String>, Option<String>);
    let row: Option<Row> = sqlx::query_as(
        "SELECT id, name, ispublic, description, notes, \
                to_char(updated, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated \
         FROM faq_category WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal("FAQ category lookup failed"))?;

    let (id, name, ispublic, description, notes, updated) =
        row.ok_or_else(|| ApiError::not_found("FAQ category not found"))?;

    Ok(Json(json!({
        "id": id,
        "name": name,
        "ispublic": ispublic,
        "description": description,
        "notes": notes.unwrap_or_default(),
        "updated": updated,
    })))
}

// ---------------------------------------------------------------------------
// POST / PUT — create / edit.
// ---------------------------------------------------------------------------

/// Create/edit FAQ-category request body.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryWriteRequest {
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_true")]
    pub ispublic: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Validate the category name (FS-032.14). Returns the trimmed name.
fn validate_name(body: &CategoryWriteRequest) -> Result<String, ApiError> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::validation("Category name required")
            .with_field("name", "Category name required"));
    }
    Ok(name)
}

/// `POST /api/staff/faq-categories` — create (admin OR flag + CSRF).
///
/// @implements FS-032.14: create validation (name required).
pub async fn create_category(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Json(body): Json<CategoryWriteRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin_or_permission(&state, &session, CAP).await?;
    let pool = pool(&state)?;

    let name = validate_name(&body)?;

    let id: i32 = sqlx::query_scalar(
        "INSERT INTO faq_category (name, ispublic, description, notes) \
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(&name)
    .bind(body.ispublic)
    .bind(body.description.as_deref().unwrap_or(""))
    .bind(body.notes.as_deref())
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::warn!(error = %e, "create_category: insert failed");
        ApiError::internal("Could not create the FAQ category")
    })?;

    Ok(Json(json!({ "id": id })))
}

/// `PUT /api/staff/faq-categories/:id` — edit; 404 when absent.
///
/// @implements FS-032.14: edit validation (name required).
/// @implements KL-032.11: missing id returns 404, not a hollow object.
pub async fn update_category(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i32>,
    Json(body): Json<CategoryWriteRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin_or_permission(&state, &session, CAP).await?;
    let pool = pool(&state)?;

    let exists: Option<i32> = sqlx::query_scalar("SELECT id FROM faq_category WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("FAQ category lookup failed"))?;
    if exists.is_none() {
        return Err(ApiError::not_found("FAQ category not found"));
    }

    let name = validate_name(&body)?;

    sqlx::query(
        "UPDATE faq_category SET name = $2, ispublic = $3, description = $4, notes = $5, \
                updated = now() WHERE id = $1",
    )
    .bind(id)
    .bind(&name)
    .bind(body.ispublic)
    .bind(body.description.as_deref().unwrap_or(""))
    .bind(body.notes.as_deref())
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::warn!(error = %e, "update_category: update failed");
        ApiError::internal("Could not update the FAQ category")
    })?;

    Ok(Json(json!({ "id": id })))
}

// ---------------------------------------------------------------------------
// DELETE /api/staff/faq-categories/:id — plain delete.
// ---------------------------------------------------------------------------

/// `DELETE /api/staff/faq-categories/:id` — plain delete (admin OR flag + CSRF).
///
/// No `faq` articles exist yet (M7), so the documented cascade to contained
/// articles' associations is defined-but-inert.
///
/// @implements FS-032.16: plain delete (article cascade defined-but-inert).
pub async fn delete_category(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i32>,
) -> Result<Json<Value>, ApiError> {
    require_admin_or_permission(&state, &session, CAP).await?;
    let pool = pool(&state)?;

    let n = sqlx::query("DELETE FROM faq_category WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|_| ApiError::internal("Could not delete the FAQ category"))?
        .rows_affected();
    if n == 0 {
        return Err(ApiError::not_found("FAQ category not found"));
    }

    Ok(Json(json!({ "affected": 1, "message": "FAQ category deleted" })))
}

// ---------------------------------------------------------------------------
// POST /api/staff/faq-categories/mass — delete / makepublic / makeprivate.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassCategoryRequest {
    pub action: String,
    #[serde(default)]
    pub ids: Vec<i32>,
}

/// `POST /api/staff/faq-categories/mass` — delete / makepublic / makeprivate.
///
/// @implements FS-032.13: FAQ category mass actions.
pub async fn mass_categories(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Json(body): Json<MassCategoryRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin_or_permission(&state, &session, CAP).await?;
    let pool = pool(&state)?;

    let action = body.action.trim().to_lowercase();
    if !matches!(action.as_str(), "delete" | "makepublic" | "makeprivate") {
        return Err(ApiError::validation("Unknown mass action"));
    }
    if body.ids.is_empty() {
        return Err(ApiError::validation("You must select at least one category."));
    }
    let ids: Vec<i32> = {
        let mut v = body.ids.clone();
        v.sort_unstable();
        v.dedup();
        v
    };

    let (affected, message) = match action.as_str() {
        "delete" => {
            let n = sqlx::query("DELETE FROM faq_category WHERE id = ANY($1)")
                .bind(&ids)
                .execute(pool)
                .await
                .map_err(|_| ApiError::internal("Delete failed"))?
                .rows_affected() as i64;
            (n, "Selected categories deleted".to_string())
        }
        "makepublic" => {
            let n = sqlx::query("UPDATE faq_category SET ispublic = true, updated = now() WHERE id = ANY($1)")
                .bind(&ids)
                .execute(pool)
                .await
                .map_err(|_| ApiError::internal("Update failed"))?
                .rows_affected() as i64;
            (n, "Selected categories made public".to_string())
        }
        "makeprivate" => {
            let n = sqlx::query("UPDATE faq_category SET ispublic = false, updated = now() WHERE id = ANY($1)")
                .bind(&ids)
                .execute(pool)
                .await
                .map_err(|_| ApiError::internal("Update failed"))?
                .rows_affected() as i64;
            (n, "Selected categories made internal".to_string())
        }
        _ => unreachable!(),
    };

    Ok(Json(json!({ "affected": affected, "message": message })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_blank_name() {
        let body = CategoryWriteRequest::default();
        assert!(validate_name(&body).unwrap_err().fields.contains_key("name"));
    }

    #[test]
    fn validate_trims_name() {
        let body = CategoryWriteRequest {
            name: "  General  ".into(),
            ..Default::default()
        };
        assert_eq!(validate_name(&body).unwrap(), "General");
    }
}
