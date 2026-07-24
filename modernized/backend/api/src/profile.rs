//! Own-profile + staff directory endpoints (TS-M4-B5) — FS-031.9/.10/.13.
//!
//! These are **staff-realm** routes (any authenticated staff), NOT admin-gated:
//!
//! * `GET/PUT /api/staff/profile` — the authenticated staff member's own record
//!   (username read-only; a submitted foreign id is a tamper error, EC-031-006).
//! * `PUT /api/staff/profile/password` — current + new + confirm, in the pinned
//!   FS-031.13 validation order.
//! * `GET /api/staff/directory` — directory-visible staff only, `q` shape-matched
//!   per BS-031-030.
//!
//! @implements FS-031.9: staff directory (browse + search).
//! @implements FS-031.10: own profile view + edit.
//! @implements FS-031.13: own-profile password change (ordered validation).
//! @implements BS-031-013: password rules + clear-forced-change on success.
//! @implements BS-031-030: directory search-term interpretation.
//! @implements BS-031-031: directory lists only directory-visible staff.
//! @implements BS-031-032: profile edits self only (tamper guard).

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use ost_core::validation::is_email;
use ost_core::{verify_password, ApiError};

use crate::auth::realm::StaffSession;
use crate::auth::StaffCsrf;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// GET /api/staff/profile
// ---------------------------------------------------------------------------

/// The own-profile record. A derived `FromRow` struct (rather than a tuple) so it
/// can carry more than the 16 columns sqlx tuples support.
#[derive(Debug, sqlx::FromRow)]
struct ProfileRow {
    staff_id: i32,
    username: String,
    firstname: String,
    lastname: String,
    email: String,
    phone: String,
    phone_ext: String,
    mobile: String,
    signature: String,
    timezone_id: Option<i32>,
    daylight_saving: bool,
    max_page_size: i32,
    auto_refresh_rate: i32,
    default_signature_type: String,
    default_paper_size: String,
    change_passwd: bool,
    onvacation: bool,
    dept_id: i32,
}

/// `GET /api/staff/profile` — the authenticated staff member's own record.
///
/// @implements FS-031.10: loads the session's staff record; username read-only.
/// @implements FS-031.11: surfaces forced-change + vacation flags.
pub async fn get_profile(
    State(state): State<AppState>,
    session: StaffSession,
) -> Result<Json<Value>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Profile is unavailable"))?;

    let row: Option<ProfileRow> = sqlx::query_as(
        "SELECT staff_id, username, firstname, lastname, email, phone, phone_ext, mobile, signature, \
                timezone_id, COALESCE(daylight_saving, false) AS daylight_saving, \
                COALESCE(max_page_size, 0) AS max_page_size, \
                COALESCE(auto_refresh_rate, 0) AS auto_refresh_rate, \
                COALESCE(default_signature_type, 'none') AS default_signature_type, \
                COALESCE(default_paper_size, 'Letter') AS default_paper_size, \
                COALESCE(change_passwd, false) AS change_passwd, \
                COALESCE(onvacation, false) AS onvacation, dept_id \
         FROM staff WHERE staff_id = $1",
    )
    .bind(session.staff_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal("Profile lookup failed"))?;

    let r = row.ok_or_else(|| ApiError::unauthenticated("Session expired or invalid"))?;

    Ok(Json(json!({
        "id": r.staff_id,
        "username": r.username,
        "firstname": r.firstname,
        "lastname": r.lastname,
        "email": r.email,
        "phone": r.phone,
        "phone_ext": r.phone_ext,
        "mobile": r.mobile,
        "signature": r.signature,
        "timezone_id": r.timezone_id,
        "daylight_saving": r.daylight_saving,
        "max_page_size": r.max_page_size,
        "auto_refresh_rate": r.auto_refresh_rate,
        "default_signature_type": r.default_signature_type,
        "default_paper_size": r.default_paper_size,
        "change_passwd": r.change_passwd,
        "onvacation": r.onvacation,
        "dept_id": r.dept_id,
    })))
}

// ---------------------------------------------------------------------------
// PUT /api/staff/profile
// ---------------------------------------------------------------------------

/// Profile-update body. `id`, when present, must equal the session staff id
/// (tamper guard, EC-031-006). Username is never updatable here.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileRequest {
    #[serde(default)]
    pub id: Option<i32>,
    #[serde(default)]
    pub firstname: Option<String>,
    #[serde(default)]
    pub lastname: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub phone_ext: Option<String>,
    #[serde(default)]
    pub mobile: Option<String>,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub timezone_id: Option<i32>,
    #[serde(default)]
    pub daylight_saving: Option<bool>,
    #[serde(default)]
    pub max_page_size: Option<i32>,
    #[serde(default)]
    pub auto_refresh_rate: Option<i32>,
    #[serde(default)]
    pub default_signature_type: Option<String>,
    #[serde(default)]
    pub default_paper_size: Option<String>,
    /// Present-but-ignored: the profile screen never changes the username.
    #[serde(default)]
    pub username: Option<String>,
}

/// `PUT /api/staff/profile` — edit the session staff member's own profile.
///
/// @implements FS-031.10: contact/prefs/signature/timezone update; username
///   read-only; on save the session's timezone offset is refreshed (DB-derived).
/// @implements BS-031-032 / EC-031-006: a submitted foreign id → Action Denied.
pub async fn update_profile(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Json(body): Json<UpdateProfileRequest>,
) -> Result<Json<Value>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Profile is unavailable"))?;

    // Tamper guard: a submitted id must match the session's staff id.
    if let Some(submitted) = body.id {
        if submitted != session.staff_id {
            return Err(ApiError::forbidden("Action Denied"));
        }
    }

    let mut err = ApiError::validation(
        "Profile update error. Try correcting the errors below and try again!",
    );
    if let Some(ref v) = body.firstname {
        if v.trim().is_empty() {
            err = err.with_field("firstname", "First name required");
        }
    }
    if let Some(ref v) = body.lastname {
        if v.trim().is_empty() {
            err = err.with_field("lastname", "Last name required");
        }
    }
    if let Some(ref v) = body.email {
        let e = v.trim();
        if e.is_empty() || !is_email(e) {
            err = err.with_field("email", "Valid email required");
        }
    }
    if !err.fields.is_empty() {
        return Err(err);
    }

    // Username is intentionally never applied (read-only on the profile).
    sqlx::query(
        "UPDATE staff SET \
            firstname = COALESCE($2, firstname), \
            lastname = COALESCE($3, lastname), \
            email = COALESCE($4, email), \
            phone = COALESCE($5, phone), \
            phone_ext = COALESCE($6, phone_ext), \
            mobile = COALESCE($7, mobile), \
            signature = COALESCE($8, signature), \
            timezone_id = COALESCE($9, timezone_id), \
            daylight_saving = COALESCE($10, daylight_saving), \
            max_page_size = COALESCE($11, max_page_size), \
            auto_refresh_rate = COALESCE($12, auto_refresh_rate), \
            default_signature_type = COALESCE($13, default_signature_type), \
            default_paper_size = COALESCE($14, default_paper_size), \
            updated = now() \
         WHERE staff_id = $1",
    )
    .bind(session.staff_id)
    .bind(body.firstname.as_deref().map(str::trim))
    .bind(body.lastname.as_deref().map(str::trim))
    .bind(body.email.as_deref().map(str::trim))
    .bind(body.phone.as_deref().map(str::trim))
    .bind(body.phone_ext.as_deref().map(str::trim))
    .bind(body.mobile.as_deref().map(str::trim))
    .bind(body.signature.as_deref())
    .bind(body.timezone_id)
    .bind(body.daylight_saving)
    .bind(body.max_page_size)
    .bind(body.auto_refresh_rate)
    .bind(body.default_signature_type.as_deref())
    .bind(body.default_paper_size.as_deref())
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::warn!(error = %e, "update_profile: update failed");
        ApiError::internal("Could not update profile")
    })?;

    // Return the refreshed record (subsequent reads reflect the new timezone).
    get_profile(State(state), session).await
}

// ---------------------------------------------------------------------------
// PUT /api/staff/profile/password
// ---------------------------------------------------------------------------

/// Password-change body. `current` is required on the current-password path
/// (the only path implemented; the reset-token path is deferred — see the note
/// in TS-M4-B5, no M1 reset-token store yet).
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    #[serde(default)]
    pub current: String,
    #[serde(default, alias = "new_password", alias = "newPassword")]
    pub new: String,
    #[serde(default)]
    pub confirm: String,
}

/// `PUT /api/staff/profile/password` — change the session staff member's password.
///
/// Validation order (FS-031.13 / BS-031-013): blank new → "New password required";
/// <6 → "Must be at least 6 characters"; mismatch → "Password(s) do not match";
/// then the current-password path (blank → "Current password required"; wrong →
/// "Invalid current password!"; new==current → "New password MUST be different
/// from the current password!"). On success, forced-change is cleared and
/// `passwdreset` is stamped.
///
/// @implements FS-031.13: ordered own-profile password validation.
/// @implements BS-031-013: clear forced-change + record reset on success.
pub async fn change_password(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<Json<Value>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Password change is unavailable"))?;

    // 1..3: new / confirm rules first (in order).
    if body.new.is_empty() {
        return Err(ApiError::validation("New password required").with_field("new", "New password required"));
    }
    if body.new.len() < 6 {
        return Err(ApiError::validation("Must be at least 6 characters")
            .with_field("new", "Must be at least 6 characters"));
    }
    if body.new != body.confirm {
        return Err(ApiError::validation("Password(s) do not match")
            .with_field("confirm", "Password(s) do not match"));
    }

    // Current-password path (reset-token path deferred).
    if body.current.is_empty() {
        return Err(ApiError::validation("Current password required")
            .with_field("current", "Current password required"));
    }

    let passwd: Option<String> = sqlx::query_scalar("SELECT passwd FROM staff WHERE staff_id = $1")
        .bind(session.staff_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("Password lookup failed"))?;
    let passwd = passwd.ok_or_else(|| ApiError::unauthenticated("Session expired or invalid"))?;

    let ok = verify_password(&body.current, &passwd).unwrap_or(false);
    if !ok {
        return Err(ApiError::validation("Invalid current password!")
            .with_field("current", "Invalid current password!"));
    }
    if body.new.eq_ignore_ascii_case(&body.current) {
        return Err(ApiError::validation("New password MUST be different from the current password!")
            .with_field("new", "New password MUST be different from the current password!"));
    }

    let new_hash = ost_core::hash_password(&body.new)
        .map_err(|_| ApiError::internal("Password hashing failed"))?;

    // On success: store the new hash, clear forced-change, stamp passwdreset.
    sqlx::query(
        "UPDATE staff SET passwd = $2, change_passwd = false, passwdreset = now(), updated = now() \
         WHERE staff_id = $1",
    )
    .bind(session.staff_id)
    .bind(&new_hash)
    .execute(pool)
    .await
    .map_err(|_| ApiError::internal("Could not change password"))?;

    Ok(Json(json!({ "ok": true })))
}

// ---------------------------------------------------------------------------
// GET /api/staff/directory
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
pub struct DirectoryParams {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub did: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub order: Option<String>,
    #[serde(default)]
    pub page: Option<String>,
}

fn dir_sort_expr(sort: Option<&str>) -> &'static str {
    match sort.unwrap_or("name") {
        "email" => "s.email",
        "dept" => "d.dept_name",
        "phone" => "s.phone",
        "mobile" => "s.mobile",
        "ext" => "s.phone_ext",
        "created" => "s.created",
        "login" => "s.lastlogin",
        _ => "s.firstname, s.lastname",
    }
}

/// Determine the SQL predicate for a directory search term (BS-031-030).
/// Returns `(clause, bind_value)` where the clause references `$1`.
fn directory_q_clause(term: &str) -> Option<(String, String)> {
    let t = term.trim();
    if t.is_empty() {
        return None;
    }
    // Purely numeric → phone / extension / mobile.
    if t.chars().all(|c| c.is_ascii_digit()) {
        return Some((
            "(s.phone ILIKE $1 OR s.phone_ext ILIKE $1 OR s.mobile ILIKE $1)".to_string(),
            format!("%{t}%"),
        ));
    }
    // Contains @ and is a valid email → exact email (case-insensitive).
    if t.contains('@') && is_email(t) {
        return Some(("lower(s.email) = lower($1)".to_string(), t.to_string()));
    }
    // Otherwise → email OR lastname OR firstname substring (case-insensitive).
    Some((
        "(s.email ILIKE $1 OR s.lastname ILIKE $1 OR s.firstname ILIKE $1)".to_string(),
        format!("%{t}%"),
    ))
}

/// `GET /api/staff/directory` — directory-visible staff (any staff, not admin).
///
/// @implements FS-031.9: directory browse + search + pagination.
/// @implements BS-031-030: search-term shape interpretation.
/// @implements BS-031-031: only directory-visible staff (isvisible=true).
pub async fn directory(
    State(state): State<AppState>,
    session: StaffSession,
    Query(params): Query<DirectoryParams>,
) -> Result<Json<Value>, ApiError> {
    // Any authenticated staff — the StaffSession extractor already gated this.
    let _ = &session;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Directory is unavailable"))?;

    let mut conds: Vec<String> = vec!["s.isvisible = true".to_string()];
    if let Some(did) = params.did.as_deref().and_then(|s| s.trim().parse::<i32>().ok()) {
        conds.push(format!("s.dept_id = {did}"));
    }
    let q_clause = params.q.as_deref().and_then(directory_q_clause);
    if let Some((ref clause, _)) = q_clause {
        conds.push(clause.clone());
    }
    let where_clause = conds.join(" AND ");

    let order = match params.order.as_deref().map(|s| s.to_uppercase()) {
        Some(ref o) if o == "DESC" => "DESC",
        _ => "ASC",
    };
    let order_by = format!("{} {order}, s.staff_id ASC", dir_sort_expr(params.sort.as_deref()));

    let per_page: i64 = 25;
    let page = params
        .page
        .as_deref()
        .and_then(|s| s.trim().parse::<i64>().ok())
        .filter(|p| *p > 0)
        .unwrap_or(1);
    let offset = (page - 1) * per_page;

    let bind = q_clause.as_ref().map(|(_, v)| v.clone());

    let count_sql = format!("SELECT COUNT(*) FROM staff s WHERE {where_clause}");
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
    if let Some(ref b) = bind {
        count_q = count_q.bind(b);
    }
    let total: i64 = count_q
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal("Directory count failed"))?;

    let rows_sql = format!(
        "SELECT s.staff_id, s.username, s.firstname, s.lastname, d.dept_name, \
                s.email, s.phone, s.phone_ext, s.mobile \
         FROM staff s \
         JOIN department d ON d.dept_id = s.dept_id \
         WHERE {where_clause} \
         ORDER BY {order_by} \
         LIMIT {per_page} OFFSET {offset}"
    );
    let mut rows_q = sqlx::query_as::<
        _,
        (i32, String, String, String, String, String, String, String, String),
    >(&rows_sql);
    if let Some(ref b) = bind {
        rows_q = rows_q.bind(b);
    }
    let rows = rows_q
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::internal("Directory list failed"))?;

    let staff: Vec<Value> = rows
        .into_iter()
        .map(|(id, username, fname, lname, dept, email, phone, ext, mobile)| {
            let full = format!("{} {}", fname.trim(), lname.trim());
            let name = if full.trim().is_empty() { username } else { full.trim().to_string() };
            json!({
                "id": id,
                "name": name,
                "dept_name": dept,
                "email": email,
                "phone": phone,
                "phone_ext": ext,
                "mobile": mobile,
            })
        })
        .collect();

    Ok(Json(json!({
        "staff": staff,
        "pagination": { "page": page, "per_page": per_page, "total": total },
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_term_matches_phone_fields() {
        let (clause, bind) = directory_q_clause("5551234").unwrap();
        assert!(clause.contains("s.phone ILIKE $1"));
        assert!(clause.contains("s.mobile ILIKE $1"));
        assert_eq!(bind, "%5551234%");
    }

    #[test]
    fn email_term_matches_exact_email() {
        let (clause, bind) = directory_q_clause("Agent@Example.com").unwrap();
        assert_eq!(clause, "lower(s.email) = lower($1)");
        assert_eq!(bind, "Agent@Example.com");
    }

    #[test]
    fn name_term_matches_name_and_email_substring() {
        let (clause, bind) = directory_q_clause("lovel").unwrap();
        assert!(clause.contains("s.lastname ILIKE $1"));
        assert!(clause.contains("s.firstname ILIKE $1"));
        assert_eq!(bind, "%lovel%");
    }

    #[test]
    fn empty_term_yields_no_clause() {
        assert!(directory_q_clause("   ").is_none());
    }
}
