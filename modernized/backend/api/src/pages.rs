//! Admin site-page management + content/config read endpoints (TS-M4-F1) —
//! FS-033.9/.10/.13/.14/.15/.16.
//!
//! Admin-gated CRUD over the `page` table with the BS-033.9 in-use guards (a page
//! bound as landing/offline/thank-you OR referenced by a help topic refuses both
//! delete and disable), plus:
//! * `GET /api/staff/admin/content/ticket_variables` — the static ticket-variable
//!   reference card (FS-033.9), admin-gated;
//! * `GET /api/config/scp` — the staff-config bundle (FS-033.10), admin-gated per
//!   BS-033.1;
//! * `GET /api/pages/:slug` — the UNAUTHENTICATED public serving of an active
//!   `type='other'` page by slugified name.
//!
//! @implements FS-033.13: page list (with in-use flag).
//! @implements FS-033.14: page create/edit validation (name unique, type set).
//! @implements FS-033.15: page bulk enable/disable/delete.
//! @implements FS-033.16: single-page in-use guards (disable + delete refused).
//! @implements BS-033.9: in-use = config binding OR help_topic reference.
//! @implements FS-033.9: ticket-variable reference content.
//! @implements FS-033.10 / BS-033.1: staff-config bundle, admin-gated.

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

/// The three `*_page_id` config keys that bind a page as a special site page.
/// A page whose id string appears in any of these is "in use" (BS-033.9).
pub const PAGE_BINDING_KEYS: [&str; 3] =
    ["landing_page_id", "offline_page_id", "thank-you_page_id"];

/// The valid `page.type` literals (mirrors the table CHECK constraint).
const PAGE_TYPES: [&str; 4] = ["landing", "offline", "thank-you", "other"];

/// Whether a page is in use: bound via a `*_page_id` config key (string compare —
/// config values are text) OR referenced by `help_topic.page_id`.
async fn is_page_in_use(pool: &sqlx::postgres::PgPool, id: i32) -> Result<bool, ApiError> {
    let in_use: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
             SELECT 1 FROM config \
             WHERE key = ANY($2) AND value = $1::text \
         ) OR EXISTS( \
             SELECT 1 FROM help_topic WHERE page_id = $1 \
         )",
    )
    .bind(id)
    .bind(&PAGE_BINDING_KEYS[..])
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal("In-use check failed"))?;
    Ok(in_use)
}

// ---------------------------------------------------------------------------
// GET /api/staff/admin/pages — list.
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageListParams {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub order: Option<String>,
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub per_page: Option<i64>,
}

fn page_sort_expr(sort: Option<&str>) -> &'static str {
    match sort {
        Some("type") => "p.type",
        Some("status") | Some("isactive") => "p.isactive",
        Some("created") => "p.created",
        Some("updated") => "p.updated",
        _ => "p.name",
    }
}

/// `GET /api/staff/admin/pages` — every page (admin), with the computed `in_use`
/// flag, paginated.
///
/// @implements FS-033.13: page list with in-use flag.
pub async fn list_pages(
    State(state): State<AppState>,
    session: StaffSession,
    Query(params): Query<PageListParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Page list is unavailable"))?;

    let order = match params.order.as_deref().map(|s| s.to_uppercase()) {
        Some(ref o) if o == "DESC" => "DESC",
        _ => "ASC",
    };
    let order_by = format!("{} {order}, p.id ASC", page_sort_expr(params.sort.as_deref()));
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(25).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM page")
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal("Page count failed"))?;

    let sql = format!(
        "SELECT p.id, p.name, p.type, p.isactive, \
                (EXISTS(SELECT 1 FROM config c WHERE c.key = ANY($1) AND c.value = p.id::text) \
                 OR EXISTS(SELECT 1 FROM help_topic ht WHERE ht.page_id = p.id)) AS in_use, \
                to_char(p.created, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created, \
                to_char(p.updated, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated \
         FROM page p ORDER BY {order_by} LIMIT $2 OFFSET $3"
    );
    // (id, name, type, isactive, in_use, created, updated)
    type PageRow = (i32, String, String, bool, bool, Option<String>, Option<String>);
    let rows: Vec<PageRow> = sqlx::query_as(&sql)
        .bind(&PAGE_BINDING_KEYS[..])
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::internal("Page list failed"))?;

    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, name, ptype, isactive, in_use, created, updated)| {
            json!({
                "id": id,
                "name": name,
                "type": ptype,
                "isactive": isactive,
                "in_use": in_use,
                "created": created,
                "updated": updated,
            })
        })
        .collect();

    Ok(Json(json!({
        "items": items,
        "pagination": { "page": page, "per_page": per_page, "total": total },
    })))
}

// ---------------------------------------------------------------------------
// POST / PUT /api/staff/admin/pages — create / edit.
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageWriteRequest {
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "type")]
    pub page_type: String,
    #[serde(default)]
    pub body: String,
    #[serde(default = "default_true")]
    pub isactive: bool,
    #[serde(default)]
    pub notes: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Validate the page name + type (FS-033.14). Returns the trimmed name + type.
fn validate_page_fields(body: &PageWriteRequest) -> Result<(String, String), ApiError> {
    let mut err = ApiError::validation("Please correct the errors below");
    let name = body.name.trim().to_string();
    if name.is_empty() {
        err = err.with_field("name", "Name required");
    }
    let ptype = body.page_type.trim().to_string();
    if !PAGE_TYPES.contains(&ptype.as_str()) {
        err = err.with_field("type", "Type must be one of landing, offline, thank-you, other");
    }
    if !err.fields.is_empty() {
        return Err(err);
    }
    Ok((name, ptype))
}

/// Map a unique-violation on `page_name_key` (23505) to a 422 keyed on `name`.
fn map_page_write_error(e: sqlx::Error, context: &'static str) -> ApiError {
    if let sqlx::Error::Database(db) = &e {
        if db.code().as_deref() == Some("23505") && db.constraint() == Some("page_name_key") {
            return ApiError::validation("A page with this name already exists")
                .with_field("name", "A page with this name already exists");
        }
    }
    tracing::warn!(error = %e, context, "page write failed");
    ApiError::internal("Could not save the page")
}

/// `POST /api/staff/admin/pages` — create a page (admin + CSRF).
///
/// @implements FS-033.14 / BS-033.10: create (name required + unique, type set).
pub async fn create_page(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    headers: http::HeaderMap,
    Json(body): Json<PageWriteRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Page create is unavailable"))?;

    let (name, ptype) = validate_page_fields(&body)?;

    let id: i32 = sqlx::query_scalar(
        "INSERT INTO page (name, type, body, isactive, notes) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(&name)
    .bind(&ptype)
    .bind(&body.body)
    .bind(body.isactive)
    .bind(body.notes.as_deref())
    .fetch_one(pool)
    .await
    .map_err(|e| map_page_write_error(e, "create_page"))?;

    log(
        pool,
        LogType::Debug,
        "Page created",
        format!("Page '{name}' (#{id}) created by staff #{}", session.staff_id),
        client_ip(&headers),
    )
    .await;

    Ok(Json(json!({ "id": id })))
}

/// `PUT /api/staff/admin/pages/:id` — edit a page (admin + CSRF).
///
/// Disable guard (FS-033.16): setting `isactive = false` on an in-use page is
/// refused with a 422 carrying the pinned message.
///
/// @implements FS-033.14: edit validation.
/// @implements FS-033.16: an in-use page cannot be disabled.
pub async fn update_page(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i32>,
    Json(body): Json<PageWriteRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Page edit is unavailable"))?;

    let exists: Option<i32> = sqlx::query_scalar("SELECT id FROM page WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("Page lookup failed"))?;
    if exists.is_none() {
        return Err(ApiError::not_found("Page not found"));
    }

    let (name, ptype) = validate_page_fields(&body)?;

    // BS-033.9 / FS-033.16: an in-use page cannot be disabled.
    if !body.isactive && is_page_in_use(pool, id).await? {
        return Err(ApiError::validation("A page currently in-use CANNOT be disabled!")
            .with_field("isactive", "A page currently in-use CANNOT be disabled!"));
    }

    sqlx::query(
        "UPDATE page SET name = $2, type = $3, body = $4, isactive = $5, notes = $6, \
                updated = now() WHERE id = $1",
    )
    .bind(id)
    .bind(&name)
    .bind(&ptype)
    .bind(&body.body)
    .bind(body.isactive)
    .bind(body.notes.as_deref())
    .execute(pool)
    .await
    .map_err(|e| map_page_write_error(e, "update_page"))?;

    Ok(Json(json!({ "id": id })))
}

// ---------------------------------------------------------------------------
// POST /api/staff/admin/pages/mass — enable / disable / delete.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassPageRequest {
    pub action: String,
    #[serde(default)]
    pub ids: Vec<i32>,
}

/// `POST /api/staff/admin/pages/mass` — enable / disable / delete (admin + CSRF).
///
/// Disable + delete refuse in-use pages (partial "N of M" message). A permitted
/// delete resets any `help_topic.page_id` pointing at the page back to `0` (the
/// "no page" sentinel) in the same transaction.
///
/// @implements FS-033.15: bulk enable/disable/delete.
/// @implements FS-033.16 / BS-033.9: in-use pages cannot be disabled/deleted.
pub async fn mass_pages(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    headers: http::HeaderMap,
    Json(body): Json<MassPageRequest>,
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
        return Err(ApiError::validation("You must select at least one page."));
    }
    let ids: Vec<i32> = {
        let mut v = body.ids.clone();
        v.sort_unstable();
        v.dedup();
        v
    };
    let requested = ids.len() as i64;

    let (affected, message) = match action.as_str() {
        "enable" => {
            let n = sqlx::query("UPDATE page SET isactive = true, updated = now() WHERE id = ANY($1)")
                .bind(&ids)
                .execute(pool)
                .await
                .map_err(|_| ApiError::internal("Enable failed"))?
                .rows_affected() as i64;
            (n, "Selected pages enabled".to_string())
        }
        "disable" => {
            // Only pages NOT in use may be disabled.
            let mut targets: Vec<i32> = Vec::new();
            for &id in &ids {
                if !is_page_in_use(pool, id).await? {
                    targets.push(id);
                }
            }
            let n = if targets.is_empty() {
                0
            } else {
                sqlx::query("UPDATE page SET isactive = false, updated = now() WHERE id = ANY($1)")
                    .bind(&targets)
                    .execute(pool)
                    .await
                    .map_err(|_| ApiError::internal("Disable failed"))?
                    .rows_affected() as i64
            };
            let msg = if n == requested {
                "Selected pages disabled".to_string()
            } else if n == 0 {
                "A page currently in-use CANNOT be disabled!".to_string()
            } else {
                format!("{n} of {requested} selected pages disabled")
            };
            (n, msg)
        }
        "delete" => {
            let mut deleted = 0i64;
            for &id in &ids {
                if is_page_in_use(pool, id).await? {
                    continue;
                }
                // Permitted delete: reset any help-topic reference to 0 first,
                // then delete the page — one transaction.
                let mut tx = pool
                    .begin()
                    .await
                    .map_err(|_| ApiError::internal("Delete failed"))?;
                sqlx::query("UPDATE help_topic SET page_id = 0 WHERE page_id = $1")
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|_| ApiError::internal("Delete failed (topic reset)"))?;
                let n = sqlx::query("DELETE FROM page WHERE id = $1")
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|_| ApiError::internal("Delete failed"))?
                    .rows_affected();
                tx.commit()
                    .await
                    .map_err(|_| ApiError::internal("Delete failed"))?;
                deleted += n as i64;
            }
            let msg = if deleted == requested {
                "Selected pages deleted successfully".to_string()
            } else if deleted == 0 {
                "Unable to delete selected pages (a page currently in-use cannot be deleted)".to_string()
            } else {
                format!("{deleted} of {requested} selected pages deleted")
            };
            (deleted, msg)
        }
        _ => unreachable!(),
    };

    // TS-M4-G1: audit an admin CRUD delete (Debug, best-effort).
    if action == "delete" && affected > 0 {
        log(
            pool,
            LogType::Debug,
            "Pages deleted",
            format!("{affected} page(s) deleted by staff #{}", session.staff_id),
            client_ip(&headers),
        )
        .await;
    }

    Ok(Json(json!({ "affected": affected, "message": message })))
}

// ---------------------------------------------------------------------------
// GET /api/staff/admin/content/ticket_variables — static reference (FS-033.9).
// ---------------------------------------------------------------------------

/// `GET /api/staff/admin/content/ticket_variables` — the static ticket-variable
/// reference card (admin-gated). No DB access: a fixed catalog mirroring the M2
/// `%{token}` catalog (FS-040.11).
///
/// @implements FS-033.9: ticket-variable reference content.
pub async fn ticket_variables(
    State(state): State<AppState>,
    session: StaffSession,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;

    let base = json!([
        { "token": "ticket.id", "description": "Ticket ID (internal)" },
        { "token": "ticket.number", "description": "Ticket number" },
        { "token": "ticket.subject", "description": "Ticket subject" },
        { "token": "ticket.name", "description": "Requester name" },
        { "token": "ticket.email", "description": "Requester email address" },
        { "token": "ticket.status", "description": "Ticket status" },
        { "token": "ticket.priority", "description": "Ticket priority" },
        { "token": "ticket.dept.name", "description": "Department name" },
        { "token": "ticket.create_date", "description": "Date the ticket was created" },
        { "token": "ticket.due_date", "description": "Ticket due date (SLA)" },
    ]);
    let other = json!([
        { "token": "url", "description": "Helpdesk base URL" },
    ]);

    Ok(Json(json!({ "base": base, "other": other })))
}

// ---------------------------------------------------------------------------
// GET /api/config/scp — staff-config bundle (FS-033.10, admin-gated BS-033.1).
// ---------------------------------------------------------------------------

/// `GET /api/config/scp` — the small staff-control-panel config bundle
/// (`lock_time`, `date_format`, `max_file_uploads`). Admin-gated per BS-033.1.
///
/// @implements FS-033.10 / BS-033.1: staff-config bundle, admin-gated.
pub async fn config_scp(
    State(state): State<AppState>,
    session: StaffSession,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Config is unavailable"))?;

    let lock_time = read_config(pool, "ticket_lock_time")
        .await?
        .unwrap_or_else(|| "0".to_string());
    let date_format = read_config(pool, "date_format")
        .await?
        .unwrap_or_else(|| "m/d/Y".to_string());
    let max_file_uploads = read_config(pool, "max_file_uploads")
        .await?
        .unwrap_or_else(|| "1".to_string());

    Ok(Json(json!({
        "lock_time": lock_time,
        "date_format": date_format,
        "max_file_uploads": max_file_uploads,
    })))
}

// ---------------------------------------------------------------------------
// GET /api/pages/:slug — public serving of an active `other` page (UNAUTH).
// ---------------------------------------------------------------------------

/// Slugify a page name for the public URL: lowercase, non-alphanumeric runs
/// collapse to a single `-`, leading/trailing `-` trimmed.
#[must_use]
pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_dash = true; // start true to swallow leading separators
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// `GET /api/pages/:slug` — serve an ACTIVE `type='other'` page by slugified
/// name. UNAUTHENTICATED (no session extractor). 404 for a non-`other` type, an
/// inactive page, or an unknown slug (no existence leak beyond a plain 404).
///
/// @implements FS-033: public serving of a custom `other` page by slug.
pub async fn public_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Pages are unavailable"))?;

    let want = slugify(&slug);

    // Only active `other` pages are eligible; match by slugified name in Rust
    // (the set is small — no SQL slug function needed).
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT name, body FROM page WHERE type = 'other' AND isactive = true")
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::internal("Page lookup failed"))?;

    let matched = rows.into_iter().find(|(name, _)| slugify(name) == want);
    let (name, body) = matched.ok_or_else(|| ApiError::not_found("Page not found"))?;

    Ok(Json(json!({ "name": name, "body": body })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Terms Of Service"), "terms-of-service");
        assert_eq!(slugify("  Privacy!!Policy  "), "privacy-policy");
        assert_eq!(slugify("A/B — C"), "a-b-c");
    }

    #[test]
    fn validate_rejects_bad_type_and_empty_name() {
        let body = PageWriteRequest {
            name: "".into(),
            page_type: "bogus".into(),
            ..Default::default()
        };
        let err = validate_page_fields(&body).unwrap_err();
        assert!(err.fields.contains_key("name"));
        assert!(err.fields.contains_key("type"));
    }

    #[test]
    fn validate_accepts_known_types() {
        for t in ["landing", "offline", "thank-you", "other"] {
            let body = PageWriteRequest {
                name: "X".into(),
                page_type: t.into(),
                ..Default::default()
            };
            assert!(validate_page_fields(&body).is_ok(), "type {t}");
        }
    }
}
