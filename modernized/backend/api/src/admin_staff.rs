//! Admin staff management endpoints (TS-M4-B1) — FS-031.1/.3/.4/.5.
//!
//! Admin-gated CRUD over the `staff` roster plus the two protection rules that
//! keep an install from locking itself out:
//!
//! * **BS-031-014** — the edit form refuses to remove/lock the last active admin.
//! * **BS-031-015 / KL-031-005** — mass lock/delete refuses self-selection and
//!   refuses to leave zero active admins behind.
//!
//! Every route here first passes the [`StaffSession`] extractor (401 without a
//! session) and then [`require_admin`] (403 for a non-admin). Mutating routes use
//! the CSRF-enforcing [`StaffCsrf`] extractor.
//!
//! Single-group-per-staff is PRESERVED (KL-031-001): `staff.group_id` is a single
//! required FK; there is no multi-group model here.
//!
//! @implements FS-031.1: staff list — filter (q/did/gid/tid), sort, pagination.
//! @implements FS-031.3: create / edit a staff account.
//! @implements FS-031.4: mass enable / lock / delete.
//! @implements FS-031.5: staff field validation.
//! @implements BS-031-001: username + email uniqueness (email not a system email).
//! @implements BS-031-014: last-active-administrator protection (edit).
//! @implements BS-031-015: self-action protection (mass).
//! @implements BS-031-016: staff deletion side effects.
//! @implements BS-030-14: add / remove team membership from the profile.
//! @implements KL-031-001: single group per staff (single required FK) — PRESERVED.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use ost_core::validation::is_email;
use ost_core::ApiError;

use crate::auth::gate::require_admin;
use crate::auth::realm::StaffSession;
use crate::auth::StaffCsrf;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Shared helpers.
// ---------------------------------------------------------------------------

/// Build a display name from `firstname`/`lastname`, falling back to `username`.
fn display_name(firstname: &str, lastname: &str, username: &str) -> String {
    let full = format!("{} {}", firstname.trim(), lastname.trim());
    let full = full.trim();
    if full.is_empty() {
        username.to_string()
    } else {
        full.to_string()
    }
}

/// A pinned per-page size for the admin staff list. Read from the `max_page_size`
/// config key (FS-090.19), clamped, defaulting to 25.
async fn list_page_size(pool: &sqlx::postgres::PgPool) -> i64 {
    let raw: Option<String> =
        sqlx::query_scalar("SELECT value FROM config WHERE key = 'max_page_size'")
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    raw.and_then(|s| s.trim().parse::<i64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(25)
        .clamp(1, 1000)
}

/// Validate an optional phone/mobile number per BS-031-018: after stripping the
/// allowed punctuation `( ) - . +` and spaces, the remainder must be 7..=16
/// digits. An empty value is accepted (the field is optional).
fn phone_ok(raw: &str) -> bool {
    let t = raw.trim();
    if t.is_empty() {
        return true;
    }
    if !t
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '(' | ')' | '-' | '.' | '+' | ' '))
    {
        return false;
    }
    let digits: String = t.chars().filter(|c| c.is_ascii_digit()).collect();
    (7..=16).contains(&digits.len())
}

// ---------------------------------------------------------------------------
// GET /api/staff/admin/staff — list.
// ---------------------------------------------------------------------------

/// Query parameters for the staff list. Ids arrive as strings so a non-numeric
/// filter value is silently ignored (FS-031.1: "only numeric filter values are
/// honored") rather than failing the whole request.
#[derive(Debug, Default, Deserialize)]
pub struct StaffListParams {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub did: Option<String>,
    #[serde(default)]
    pub gid: Option<String>,
    #[serde(default)]
    pub tid: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub order: Option<String>,
    #[serde(default)]
    pub page: Option<String>,
}

fn parse_id(s: &Option<String>) -> Option<i32> {
    s.as_deref().and_then(|v| v.trim().parse::<i32>().ok())
}

/// Map a sort key to its SQL ORDER BY expression (default `name`).
fn staff_sort_expr(sort: Option<&str>) -> &'static str {
    match sort.unwrap_or("name") {
        "username" => "s.username",
        "status" => "s.isactive",
        "group" => "g.group_name",
        "dept" => "d.dept_name",
        "created" => "s.created",
        "login" => "s.lastlogin",
        // "name" and anything unknown → name (first + last).
        _ => "s.firstname, s.lastname",
    }
}

/// `GET /api/staff/admin/staff` — filtered, sorted, paginated roster (admin only).
///
/// @implements FS-031.1: list with AND-combined filters + sort + pagination.
pub async fn list_staff(
    State(state): State<AppState>,
    session: StaffSession,
    Query(params): Query<StaffListParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Staff list is unavailable"))?;

    // Build the AND-combined WHERE. Numeric ids are validated i32 → safe to
    // inline; the free-text `q` is bound as $1.
    let mut conds: Vec<String> = vec!["1 = 1".to_string()];
    if let Some(did) = parse_id(&params.did) {
        conds.push(format!("s.dept_id = {did}"));
    }
    if let Some(gid) = parse_id(&params.gid) {
        conds.push(format!("s.group_id = {gid}"));
    }
    if let Some(tid) = parse_id(&params.tid) {
        conds.push(format!(
            "EXISTS (SELECT 1 FROM team_member tm WHERE tm.staff_id = s.staff_id AND tm.team_id = {tid})"
        ));
    }
    let q = params.q.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if q.is_some() {
        conds.push(
            "(s.firstname ILIKE $1 OR s.lastname ILIKE $1 OR s.username ILIKE $1 OR s.email ILIKE $1)"
                .to_string(),
        );
    }
    let where_clause = conds.join(" AND ");

    let order = match params.order.as_deref().map(|s| s.to_uppercase()) {
        Some(ref o) if o == "DESC" => "DESC",
        _ => "ASC",
    };
    let order_by = format!("{} {order}, s.staff_id ASC", staff_sort_expr(params.sort.as_deref()));

    let per_page = list_page_size(pool).await;
    let page = params
        .page
        .as_deref()
        .and_then(|s| s.trim().parse::<i64>().ok())
        .filter(|p| *p > 0)
        .unwrap_or(1);
    let offset = (page - 1) * per_page;

    let like = q.map(|s| format!("%{s}%"));

    // Total count.
    let count_sql = format!("SELECT COUNT(*) FROM staff s WHERE {where_clause}");
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
    if let Some(ref l) = like {
        count_q = count_q.bind(l);
    }
    let total: i64 = count_q
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal("Staff count failed"))?;

    let rows_sql = format!(
        "SELECT s.staff_id, s.username, s.firstname, s.lastname, s.isactive, \
                COALESCE(s.onvacation, false) AS onvacation, \
                g.group_name, d.dept_name, \
                to_char(s.created, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created, \
                to_char(s.lastlogin, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS lastlogin \
         FROM staff s \
         JOIN groups g ON g.group_id = s.group_id \
         JOIN department d ON d.dept_id = s.dept_id \
         WHERE {where_clause} \
         ORDER BY {order_by} \
         LIMIT {per_page} OFFSET {offset}"
    );
    let mut rows_q = sqlx::query_as::<
        _,
        (
            i32,
            String,
            String,
            String,
            bool,
            bool,
            String,
            String,
            Option<String>,
            Option<String>,
        ),
    >(&rows_sql);
    if let Some(ref l) = like {
        rows_q = rows_q.bind(l);
    }
    let rows = rows_q
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::internal("Staff list failed"))?;

    let staff: Vec<Value> = rows
        .into_iter()
        .map(
            |(id, username, fname, lname, isactive, onvacation, group_name, dept_name, created, lastlogin)| {
                json!({
                    "id": id,
                    "name": display_name(&fname, &lname, &username),
                    "username": username,
                    "isactive": isactive,
                    "onvacation": onvacation,
                    "group_name": group_name,
                    "dept_name": dept_name,
                    "created": created,
                    "lastlogin": lastlogin,
                })
            },
        )
        .collect();

    Ok(Json(json!({
        "staff": staff,
        "pagination": { "page": page, "per_page": per_page, "total": total },
    })))
}

// ---------------------------------------------------------------------------
// POST /api/staff/admin/staff — create.
// ---------------------------------------------------------------------------

/// Create-staff request body (FS-031.3 fields).
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateStaffRequest {
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub firstname: String,
    #[serde(default)]
    pub lastname: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub phone_ext: String,
    #[serde(default)]
    pub mobile: String,
    #[serde(default)]
    pub password: String,
    /// Confirm-password. Accepts `passwd2` or `confirm`.
    #[serde(default, alias = "confirm")]
    pub passwd2: String,
    #[serde(default)]
    pub group_id: Option<i32>,
    #[serde(default)]
    pub dept_id: Option<i32>,
    #[serde(default)]
    pub timezone_id: Option<i32>,
    #[serde(default)]
    pub isadmin: bool,
    #[serde(default = "default_true")]
    pub isactive: bool,
    #[serde(default = "default_true")]
    pub isvisible: bool,
    #[serde(default)]
    pub onvacation: bool,
    #[serde(default)]
    pub signature: String,
    #[serde(default)]
    pub notes: Option<String>,
}

fn default_true() -> bool {
    true
}

/// `POST /api/staff/admin/staff` — validate + create a staff account (admin + CSRF).
///
/// @implements FS-031.5: field validation with per-field 422 messages.
/// @implements BS-031-001: username + email uniqueness; email ≠ system email.
/// @implements BS-031-013: temp password required (≥6), argon2id, force change.
/// @implements KL-031-001: single required group (single FK) — PRESERVED.
pub async fn create_staff(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    headers: http::HeaderMap,
    Json(body): Json<CreateStaffRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Staff create is unavailable"))?;

    let mut err = ApiError::validation("Unable to add staff. Correct any error(s) below and try again.");

    let username = body.username.trim();
    let firstname = body.firstname.trim();
    let lastname = body.lastname.trim();
    let email = body.email.trim();

    if firstname.is_empty() {
        err = err.with_field("firstname", "First name required");
    }
    if lastname.is_empty() {
        err = err.with_field("lastname", "Last name required");
    }
    if username.is_empty() {
        err = err.with_field("username", "Username required");
    }

    // Email shape.
    if email.is_empty() || !is_email(email) {
        err = err.with_field("email", "Valid email required");
    }

    // Phone / mobile shape (optional).
    if !phone_ok(&body.phone) {
        err = err.with_field("phone", "Valid number required");
    }
    if !phone_ok(&body.mobile) {
        err = err.with_field("mobile", "Valid number required");
    }

    // Password (temp required on create).
    if body.password.is_empty() {
        err = err.with_field("password", "Temp. password required");
    } else if body.password.len() < 6 {
        err = err.with_field("password", "Must be at least 6 characters");
    } else if body.password != body.passwd2 {
        err = err.with_field("passwd2", "Password(s) do not match");
    }

    // Group + department required (single group FK — KL-031-001).
    if body.group_id.is_none() {
        err = err.with_field("groupId", "Group required");
    }
    if body.dept_id.is_none() {
        err = err.with_field("deptId", "Department required");
    }

    // Uniqueness checks (only when the shape is otherwise valid).
    if !username.is_empty() {
        let clash: Option<i32> =
            sqlx::query_scalar("SELECT staff_id FROM staff WHERE lower(username) = lower($1)")
                .bind(username)
                .fetch_optional(pool)
                .await
                .map_err(|_| ApiError::internal("Staff lookup failed"))?;
        if clash.is_some() {
            err = err.with_field("username", "Username already in use");
        }
    }
    if !email.is_empty() && is_email(email) {
        let sys: Option<i32> =
            sqlx::query_scalar("SELECT id FROM email_account WHERE lower(email) = lower($1)")
                .bind(email)
                .fetch_optional(pool)
                .await
                .map_err(|_| ApiError::internal("Email lookup failed"))?;
        if sys.is_some() {
            err = err.with_field("email", "Already in-use system email");
        } else {
            let other: Option<i32> =
                sqlx::query_scalar("SELECT staff_id FROM staff WHERE lower(email) = lower($1)")
                    .bind(email)
                    .fetch_optional(pool)
                    .await
                    .map_err(|_| ApiError::internal("Email lookup failed"))?;
            if other.is_some() {
                err = err.with_field("email", "Email already in use by another staff member");
            }
        }
    }

    if !err.fields.is_empty() {
        return Err(err);
    }

    let passwd = ost_core::hash_password(&body.password)
        .map_err(|_| ApiError::internal("Password hashing failed"))?;

    // Create with change_passwd forced on (temp password → must change on login).
    let id: i32 = sqlx::query_scalar(
        "INSERT INTO staff \
           (group_id, dept_id, username, firstname, lastname, email, phone, phone_ext, mobile, \
            passwd, signature, isadmin, isactive, isvisible, onvacation, timezone_id, notes, \
            change_passwd, passwdreset) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,true,now()) \
         RETURNING staff_id",
    )
    .bind(body.group_id)
    .bind(body.dept_id)
    .bind(username)
    .bind(firstname)
    .bind(lastname)
    .bind(email)
    .bind(body.phone.trim())
    .bind(body.phone_ext.trim())
    .bind(body.mobile.trim())
    .bind(&passwd)
    .bind(body.signature.trim())
    .bind(body.isadmin)
    .bind(body.isactive)
    .bind(body.isvisible)
    .bind(body.onvacation)
    .bind(body.timezone_id)
    .bind(body.notes.as_deref())
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::warn!(error = %e, "create_staff: insert failed");
        ApiError::internal("Could not create staff")
    })?;

    // TS-M4-G1: audit the admin CRUD create (Debug, best-effort).
    ost_core::log(
        pool,
        ost_core::LogType::Debug,
        "Staff account created",
        format!("Staff account '{}' (#{id}) created by staff #{}", username, session.staff_id),
        crate::logs::client_ip(&headers),
    )
    .await;

    Ok(Json(json!({ "id": id })))
}

// ---------------------------------------------------------------------------
// PUT /api/staff/admin/staff/:id — edit.
// ---------------------------------------------------------------------------

/// Edit-staff request body. All fields optional; only the provided ones change
/// (partial update). Password resets only when a non-empty value is supplied.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStaffRequest {
    #[serde(default)]
    pub username: Option<String>,
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
    pub password: Option<String>,
    #[serde(default, alias = "confirm")]
    pub passwd2: Option<String>,
    #[serde(default)]
    pub group_id: Option<i32>,
    #[serde(default)]
    pub dept_id: Option<i32>,
    #[serde(default)]
    pub timezone_id: Option<i32>,
    #[serde(default)]
    pub isadmin: Option<bool>,
    #[serde(default)]
    pub isactive: Option<bool>,
    #[serde(default)]
    pub isvisible: Option<bool>,
    #[serde(default)]
    pub onvacation: Option<bool>,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// `PUT /api/staff/admin/staff/:id` — edit a staff account (admin + CSRF).
///
/// @implements FS-031.3/.5: edit with validation; password optional.
/// @implements BS-031-014: last-active-administrator protection (in-txn count).
pub async fn update_staff(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i32>,
    Json(body): Json<UpdateStaffRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Staff edit is unavailable"))?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal("Staff edit failed"))?;

    // Load the current row.
    let current: Option<(bool, bool, String, String)> = sqlx::query_as(
        "SELECT isadmin, isactive, username, email FROM staff WHERE staff_id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| ApiError::internal("Staff lookup failed"))?;
    let (cur_isadmin, cur_isactive, cur_username, cur_email) =
        current.ok_or_else(|| ApiError::not_found("Staff member not found"))?;

    let mut err =
        ApiError::validation("Unable to update staff. Correct any error(s) below and try again!");

    // Resolve the post-edit isadmin / isactive.
    let new_isadmin = body.isadmin.unwrap_or(cur_isadmin);
    let new_isactive = body.isactive.unwrap_or(cur_isactive);

    // BS-031-014: last-active-administrator protection. If the target IS currently
    // the only active admin and this change would remove/lock it, refuse.
    if cur_isadmin && cur_isactive && !(new_isadmin && new_isactive) {
        let other_active_admins: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM staff WHERE isadmin = true AND isactive = true AND staff_id <> $1",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("Admin count failed"))?;
        if other_active_admins == 0 {
            return Err(err.with_field(
                "isadmin",
                "Cowardly refusing to remove or lock out the only active administrator",
            ));
        }
    }

    // Field validation for the provided values.
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
    if let Some(ref v) = body.username {
        let u = v.trim();
        if u.is_empty() {
            err = err.with_field("username", "Username required");
        } else if !u.eq_ignore_ascii_case(&cur_username) {
            let clash: Option<i32> = sqlx::query_scalar(
                "SELECT staff_id FROM staff WHERE lower(username) = lower($1) AND staff_id <> $2",
            )
            .bind(u)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| ApiError::internal("Staff lookup failed"))?;
            if clash.is_some() {
                err = err.with_field("username", "Username already in use");
            }
        }
    }
    if let Some(ref v) = body.email {
        let e = v.trim();
        if e.is_empty() || !is_email(e) {
            err = err.with_field("email", "Valid email required");
        } else if !e.eq_ignore_ascii_case(&cur_email) {
            let sys: Option<i32> =
                sqlx::query_scalar("SELECT id FROM email_account WHERE lower(email) = lower($1)")
                    .bind(e)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|_| ApiError::internal("Email lookup failed"))?;
            if sys.is_some() {
                err = err.with_field("email", "Already in-use system email");
            } else {
                let other: Option<i32> = sqlx::query_scalar(
                    "SELECT staff_id FROM staff WHERE lower(email) = lower($1) AND staff_id <> $2",
                )
                .bind(e)
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|_| ApiError::internal("Email lookup failed"))?;
                if other.is_some() {
                    err = err.with_field("email", "Email already in use by another staff member");
                }
            }
        }
    }
    if let Some(ref v) = body.phone {
        if !phone_ok(v) {
            err = err.with_field("phone", "Valid number required");
        }
    }
    if let Some(ref v) = body.mobile {
        if !phone_ok(v) {
            err = err.with_field("mobile", "Valid number required");
        }
    }
    // Password reset only if provided; validate when non-empty.
    let mut new_hash: Option<String> = None;
    if let Some(ref pw) = body.password {
        if !pw.is_empty() {
            if pw.len() < 6 {
                err = err.with_field("password", "Must be at least 6 characters");
            } else if body.passwd2.as_deref().unwrap_or("") != pw {
                err = err.with_field("passwd2", "Password(s) do not match");
            } else {
                new_hash = Some(
                    ost_core::hash_password(pw)
                        .map_err(|_| ApiError::internal("Password hashing failed"))?,
                );
            }
        }
    }

    if !err.fields.is_empty() {
        return Err(err);
    }

    // Apply the update with COALESCE-style "keep current when not provided".
    sqlx::query(
        "UPDATE staff SET \
            username = COALESCE($2, username), \
            firstname = COALESCE($3, firstname), \
            lastname = COALESCE($4, lastname), \
            email = COALESCE($5, email), \
            phone = COALESCE($6, phone), \
            phone_ext = COALESCE($7, phone_ext), \
            mobile = COALESCE($8, mobile), \
            group_id = COALESCE($9, group_id), \
            dept_id = COALESCE($10, dept_id), \
            timezone_id = COALESCE($11, timezone_id), \
            isadmin = $12, \
            isactive = $13, \
            isvisible = COALESCE($14, isvisible), \
            onvacation = COALESCE($15, onvacation), \
            signature = COALESCE($16, signature), \
            notes = COALESCE($17, notes), \
            passwd = COALESCE($18, passwd), \
            passwdreset = CASE WHEN $18 IS NOT NULL THEN now() ELSE passwdreset END, \
            change_passwd = CASE WHEN $18 IS NOT NULL THEN true ELSE change_passwd END, \
            updated = now() \
         WHERE staff_id = $1",
    )
    .bind(id)
    .bind(body.username.as_deref().map(str::trim))
    .bind(body.firstname.as_deref().map(str::trim))
    .bind(body.lastname.as_deref().map(str::trim))
    .bind(body.email.as_deref().map(str::trim))
    .bind(body.phone.as_deref().map(str::trim))
    .bind(body.phone_ext.as_deref().map(str::trim))
    .bind(body.mobile.as_deref().map(str::trim))
    .bind(body.group_id)
    .bind(body.dept_id)
    .bind(body.timezone_id)
    .bind(new_isadmin)
    .bind(new_isactive)
    .bind(body.isvisible)
    .bind(body.onvacation)
    .bind(body.signature.as_deref().map(str::trim))
    .bind(body.notes.as_deref())
    .bind(new_hash.as_deref())
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::warn!(error = %e, "update_staff: update failed");
        ApiError::internal("Could not update staff")
    })?;

    tx.commit()
        .await
        .map_err(|_| ApiError::internal("Staff edit failed"))?;

    Ok(Json(json!({ "id": id })))
}

// ---------------------------------------------------------------------------
// POST /api/staff/admin/staff/mass — enable / lock / delete.
// ---------------------------------------------------------------------------

/// Mass-action request body.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassStaffRequest {
    pub action: String,
    #[serde(default)]
    pub ids: Vec<i32>,
}

/// `POST /api/staff/admin/staff/mass` — enable / lock / delete (admin + CSRF).
///
/// @implements FS-031.4: mass enable / lock / delete.
/// @implements BS-031-015: self-action protection on lock / delete.
/// @implements BS-031-016: deletion side effects (unassign + membership cleanup).
/// @implements KL-031-005 (modernised): mass lock/delete must leave ≥1 active admin.
pub async fn mass_staff(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    headers: http::HeaderMap,
    Json(body): Json<MassStaffRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Mass action is unavailable"))?;

    let action = body.action.trim().to_lowercase();
    if !matches!(action.as_str(), "enable" | "lock" | "delete") {
        return Err(ApiError::validation("Unknown mass action"));
    }
    if body.ids.is_empty() {
        return Err(ApiError::validation("You must select at least one staff member."));
    }
    let ids: Vec<i32> = {
        let mut v = body.ids.clone();
        v.sort_unstable();
        v.dedup();
        v
    };

    let is_destructive = matches!(action.as_str(), "lock" | "delete");

    // BS-031-015: self-action protection (lock / delete).
    if is_destructive && ids.contains(&session.staff_id) {
        return Err(ApiError::validation(
            "You can not disable/delete yourself - you could be the only admin!",
        ));
    }

    let (affected, message) = match action.as_str() {
        "enable" => {
            let n = sqlx::query("UPDATE staff SET isactive = true, updated = now() WHERE staff_id = ANY($1)")
                .bind(&ids)
                .execute(pool)
                .await
                .map_err(|_| ApiError::internal("Enable failed"))?
                .rows_affected() as i64;
            (n, "Selected staff activated".to_string())
        }
        // WARN-2: lock/delete run the last-active-admin count and the mutation in
        // ONE transaction, taking `SELECT ... FOR UPDATE` locks on the affected
        // admin rows first (mirroring the single-record `update_staff` guard), so
        // two concurrent mass actions can't jointly remove the last active admin.
        "lock" | "delete" => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|_| ApiError::internal("Mass action failed"))?;

            // Lock the affected active-admin rows so a concurrent destructive mass
            // action targeting the same admin(s) serializes behind this one.
            let _locked: Vec<i32> = sqlx::query_scalar(
                "SELECT staff_id FROM staff \
                 WHERE staff_id = ANY($1) AND isadmin = true AND isactive = true \
                 ORDER BY staff_id FOR UPDATE",
            )
            .bind(&ids)
            .fetch_all(&mut *tx)
            .await
            .map_err(|_| ApiError::internal("Admin lock failed"))?;

            // KL-031-005 (modernised): the mutation must not leave zero active
            // admins. Counted INSIDE the transaction, consistent with the locks.
            let remaining_admins: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM staff \
                 WHERE isadmin = true AND isactive = true AND staff_id <> ALL($1)",
            )
            .bind(&ids)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| ApiError::internal("Admin count failed"))?;
            if remaining_admins == 0 {
                // Dropping `tx` here rolls the (empty) transaction back.
                return Err(ApiError::validation(
                    "Cowardly refusing to remove or lock out the only active administrator",
                ));
            }

            let (n, msg) = if action == "lock" {
                // Safety net: never lock the acting admin (BS-031-015).
                let n = sqlx::query(
                    "UPDATE staff SET isactive = false, updated = now() \
                     WHERE staff_id = ANY($1) AND staff_id <> $2",
                )
                .bind(&ids)
                .bind(session.staff_id)
                .execute(&mut *tx)
                .await
                .map_err(|_| ApiError::internal("Lock failed"))?
                .rows_affected() as i64;
                (n, "Selected staff disabled".to_string())
            } else {
                let n = delete_staff_ops(&mut tx, &ids, session.staff_id).await?;
                (n, "Selected staff deleted successfully".to_string())
            };

            tx.commit()
                .await
                .map_err(|_| ApiError::internal("Mass action failed"))?;
            (n, msg)
        }
        _ => unreachable!(),
    };

    // TS-M4-G1: audit an admin CRUD delete (Debug, best-effort).
    if action == "delete" && affected > 0 {
        ost_core::log(
            pool,
            ost_core::LogType::Debug,
            "Staff accounts deleted",
            format!("{affected} staff account(s) deleted by staff #{}", session.staff_id),
            crate::logs::client_ip(&headers),
        )
        .await;
    }

    Ok(Json(json!({ "affected": affected, "message": message })))
}

/// Delete the selected staff (excluding the acting admin as a safety net) with
/// the BS-031-016 side effects, running on the **caller's** transaction so the
/// last-active-admin guard (WARN-2) and the delete commit as one atomic unit.
///
/// @implements BS-031-016: unassign open tickets, drop team memberships, and
///   clear the remaining staff-referencing FKs before removing the row.
async fn delete_staff_ops(
    conn: &mut sqlx::PgConnection,
    ids: &[i32],
    acting: i32,
) -> Result<i64, ApiError> {
    let targets: Vec<i32> = ids.iter().copied().filter(|&x| x != acting).collect();
    if targets.is_empty() {
        return Ok(0);
    }

    // Clear every staff-referencing FK, then delete. Open-ticket unassignment is
    // the spec-visible effect (BS-031-016); closed-ticket + thread + manager +
    // lead references are cleared too so the row can actually be removed.
    let cleanup: &[&str] = &[
        "UPDATE ticket SET staff_id = NULL WHERE staff_id = ANY($1)",
        "UPDATE ticket SET closed_by_staff_id = NULL WHERE closed_by_staff_id = ANY($1)",
        "UPDATE ticket_thread SET staff_id = NULL WHERE staff_id = ANY($1)",
        "UPDATE team SET lead_id = NULL WHERE lead_id = ANY($1)",
        "UPDATE help_topic SET staff_id = NULL WHERE staff_id = ANY($1)",
        "UPDATE department SET manager_id = NULL WHERE manager_id = ANY($1)",
        "DELETE FROM team_member WHERE staff_id = ANY($1)",
        "DELETE FROM ticket_lock WHERE staff_id = ANY($1)",
    ];
    for stmt in cleanup {
        sqlx::query(stmt)
            .bind(&targets)
            .execute(&mut *conn)
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, stmt = stmt, "delete_staff: cleanup failed");
                ApiError::internal("Delete failed")
            })?;
    }

    let n = sqlx::query("DELETE FROM staff WHERE staff_id = ANY($1)")
        .bind(&targets)
        .execute(&mut *conn)
        .await
        .map_err(|_| ApiError::internal("Delete failed"))?
        .rows_affected() as i64;

    Ok(n)
}

// ---------------------------------------------------------------------------
// Team membership from the profile (BS-030-14).
// ---------------------------------------------------------------------------

/// Add-to-team request body.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddTeamRequest {
    pub team_id: i32,
}

/// `POST /api/staff/admin/staff/:id/teams` — add a team membership (admin + CSRF).
///
/// @implements BS-030-14: add a staff member to a team from the profile.
pub async fn add_team(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i32>,
    Json(body): Json<AddTeamRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Team update is unavailable"))?;

    sqlx::query(
        "INSERT INTO team_member (team_id, staff_id) VALUES ($1, $2) \
         ON CONFLICT (team_id, staff_id) DO NOTHING",
    )
    .bind(body.team_id)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::warn!(error = %e, "add_team: insert failed");
        ApiError::internal("Could not add team membership")
    })?;

    Ok(Json(json!({ "ok": true })))
}

/// `DELETE /api/staff/admin/staff/:id/teams/:teamId` — remove a team membership
/// (admin + CSRF).
///
/// @implements BS-030-14: remove a staff member from a team.
pub async fn remove_team(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path((id, team_id)): Path<(i32, i32)>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Team update is unavailable"))?;

    let n = sqlx::query("DELETE FROM team_member WHERE team_id = $1 AND staff_id = $2")
        .bind(team_id)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|_| ApiError::internal("Could not remove team membership"))?
        .rows_affected();

    Ok(Json(json!({ "ok": true, "removed": n })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_ok_accepts_empty_and_valid() {
        assert!(phone_ok(""));
        assert!(phone_ok("  "));
        assert!(phone_ok("(555) 123-4567"));
        assert!(phone_ok("+1 555 123 4567"));
    }

    #[test]
    fn phone_ok_rejects_short_and_bad_chars() {
        assert!(!phone_ok("12345")); // <7 digits
        assert!(!phone_ok("12345678901234567")); // >16 digits
        assert!(!phone_ok("call-me")); // letters
    }

    #[test]
    fn display_name_falls_back_to_username() {
        assert_eq!(display_name("", "", "agent"), "agent");
        assert_eq!(display_name("Ada", "Lovelace", "agent"), "Ada Lovelace");
    }

    #[test]
    fn staff_sort_expr_defaults_to_name() {
        assert_eq!(staff_sort_expr(None), "s.firstname, s.lastname");
        assert_eq!(staff_sort_expr(Some("bogus")), "s.firstname, s.lastname");
        assert_eq!(staff_sort_expr(Some("username")), "s.username");
        assert_eq!(staff_sort_expr(Some("dept")), "d.dept_name");
    }
}
