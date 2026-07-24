//! Admin system-log viewer endpoints (TS-M4-G2) — FS-033.2–.8.
//!
//! Read + bulk-delete over the `syslog` table populated by the TS-M4-G1
//! write-side facility. Admin-gated per-handler.
//!
//! Route shapes (under `/api/staff/admin`, admin-gated):
//! * `GET  /logs` — filter (`type`, `from`, `to`), sort, paginate; newest-first
//!   default; the big `log` body is omitted from the list rows.
//! * `GET  /logs/:id` — one record incl. its full `log` body (404 when unknown).
//! * `POST /logs/delete` — bulk manual deletion by id.
//!
//! The grace-period purge sweep itself lives in `ost_core::log::purge_logs`
//! (TS-M4-G2); it is invoked from the dev endpoint and, later (M6), from cron.
//!
//! @implements FS-033.2: log list with type + date-range filter.
//! @implements FS-033.4/.5: sort + pagination (newest-first default).
//! @implements FS-033.8: single-record detail (full body).
//! @implements FS-033.6: bulk manual deletion.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use ost_core::ApiError;

use crate::auth::gate::require_admin;
use crate::auth::realm::StaffSession;
use crate::auth::StaffCsrf;
use crate::state::AppState;

/// Extract the client IP for an audit-log row: the first `X-Forwarded-For` hop,
/// else empty. Empty-string default is acceptable (TS-M4-G1) — the minimal
/// facility does not require a real socket peer address.
///
/// @implements TS-M4-G1: capture the client IP for a syslog row (empty default).
#[must_use]
pub fn client_ip(headers: &http::HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// GET /api/staff/admin/logs — list with filter / sort / pagination.
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
pub struct LogListParams {
    /// Filter by `log_type` (`Error` | `Warning` | `Debug`).
    #[serde(default, rename = "type")]
    pub log_type: Option<String>,
    /// Inclusive lower bound on `created` (ISO 8601 / any Postgres timestamp text).
    #[serde(default)]
    pub from: Option<String>,
    /// Inclusive upper bound on `created`.
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub order: Option<String>,
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub per_page: Option<i64>,
}

fn log_sort_expr(sort: Option<&str>) -> &'static str {
    match sort {
        Some("created") => "created",
        Some("type") | Some("log_type") => "log_type",
        Some("title") => "title",
        // Default (no/unknown sort) is newest-first by id.
        _ => "id",
    }
}

/// `GET /api/staff/admin/logs` — filtered, sorted, paginated log list (admin).
///
/// The heavy `log` body is intentionally omitted from list rows (fetched only by
/// the detail route). Default order is newest-first (`id DESC`).
///
/// @implements FS-033.2/.4/.5: filter + sort + pagination.
pub async fn list_logs(
    State(state): State<AppState>,
    session: StaffSession,
    Query(params): Query<LogListParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Log list is unavailable"))?;

    // Default order is newest-first: when no explicit sort is given, id DESC.
    let sort_col = log_sort_expr(params.sort.as_deref());
    let order = match params.order.as_deref().map(|s| s.to_uppercase()) {
        Some(ref o) if o == "ASC" => "ASC",
        Some(ref o) if o == "DESC" => "DESC",
        // No explicit order → newest-first.
        _ => "DESC",
    };
    let order_by = format!("{sort_col} {order}, id DESC");

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(25).clamp(1, 200);
    let offset = (page - 1) * per_page;

    // Optional filters via NULL-guarded bound params (text cast to timestamptz).
    let type_filter = params.log_type.clone().filter(|s| !s.trim().is_empty());
    let from = params.from.clone().filter(|s| !s.trim().is_empty());
    let to = params.to.clone().filter(|s| !s.trim().is_empty());

    let where_sql = "WHERE ($1::text IS NULL OR log_type = $1) \
                       AND ($2::timestamptz IS NULL OR created >= $2::timestamptz) \
                       AND ($3::timestamptz IS NULL OR created <= $3::timestamptz)";

    let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM syslog {where_sql}"))
        .bind(&type_filter)
        .bind(&from)
        .bind(&to)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal("Log count failed"))?;

    let list_sql = format!(
        "SELECT id, log_type, title, \
                to_char(created, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created, ip_address \
         FROM syslog {where_sql} ORDER BY {order_by} LIMIT $4 OFFSET $5"
    );
    // (id, log_type, title, created, ip_address)
    type LogRow = (i64, String, String, Option<String>, String);
    let rows: Vec<LogRow> = sqlx::query_as(&list_sql)
        .bind(&type_filter)
        .bind(&from)
        .bind(&to)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::internal("Log list failed"))?;

    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, log_type, title, created, ip)| {
            json!({
                "id": id,
                "log_type": log_type,
                "title": title,
                "created": created,
                "ip_address": ip,
            })
        })
        .collect();

    Ok(Json(json!({
        "items": items,
        "total": total,
        "page": page,
        "per_page": per_page,
    })))
}

// ---------------------------------------------------------------------------
// GET /api/staff/admin/logs/:id — single-record detail (full body).
// ---------------------------------------------------------------------------

/// `GET /api/staff/admin/logs/:id` — one log row incl. its full `log` body.
/// 404 when the id is unknown.
///
/// @implements FS-033.8: single-record detail (content).
pub async fn get_log(
    State(state): State<AppState>,
    session: StaffSession,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Log detail is unavailable"))?;

    // (id, log_type, title, log, ip_address, created)
    type DetailRow = (i64, String, String, String, String, Option<String>);
    let row: Option<DetailRow> = sqlx::query_as(
        "SELECT id, log_type, title, log, ip_address, \
                to_char(created, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created \
         FROM syslog WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal("Log detail failed"))?;

    let (id, log_type, title, log, ip, created) =
        row.ok_or_else(|| ApiError::not_found("Log entry not found"))?;

    Ok(Json(json!({
        "id": id,
        "log_type": log_type,
        "title": title,
        "log": log,
        "ip_address": ip,
        "created": created,
    })))
}

// ---------------------------------------------------------------------------
// POST /api/staff/admin/logs/delete — bulk manual deletion.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct DeleteLogsRequest {
    #[serde(default)]
    pub ids: Vec<i64>,
}

/// `POST /api/staff/admin/logs/delete` — bulk delete rows by id (admin + CSRF).
///
/// @implements FS-033.6: bulk manual deletion.
pub async fn delete_logs(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Json(body): Json<DeleteLogsRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Log delete is unavailable"))?;

    if body.ids.is_empty() {
        return Ok(Json(json!({ "affected": 0 })));
    }

    let affected = sqlx::query("DELETE FROM syslog WHERE id = ANY($1)")
        .bind(&body.ids)
        .execute(pool)
        .await
        .map_err(|_| ApiError::internal("Log delete failed"))?
        .rows_affected() as i64;

    Ok(Json(json!({ "affected": affected })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_ip_reads_first_forwarded_hop() {
        let mut h = http::HeaderMap::new();
        h.insert("x-forwarded-for", "203.0.113.7, 10.0.0.1".parse().unwrap());
        assert_eq!(client_ip(&h), "203.0.113.7");
    }

    #[test]
    fn client_ip_defaults_empty() {
        let h = http::HeaderMap::new();
        assert_eq!(client_ip(&h), "");
    }

    #[test]
    fn sort_expr_defaults_to_id() {
        assert_eq!(log_sort_expr(None), "id");
        assert_eq!(log_sort_expr(Some("created")), "created");
        assert_eq!(log_sort_expr(Some("bogus")), "id");
    }
}
