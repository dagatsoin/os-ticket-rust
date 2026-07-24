//! Canned-response management CRUD (TS-M4-H1) — FS-022.
//!
//! The write surface over the `canned_response` model (the M2 consumption path in
//! `canned.rs` is reused, not rebuilt). Gated by
//! [`require_admin_or_permission`]`(_, _, can_manage_premade)` so an admin OR a
//! delegated premade-manager may manage responses.
//!
//! Attachments reuse the M2 plumbing: [`drain_multipart`] +
//! [`load_upload_policy`] + the content-addressed blob store + an
//! `attachment_file` upsert-by-hash bound through `canned_attachment`.
//!
//! Route shapes (under `/api/staff/canned-responses`, premade-gated; mutating
//! routes CSRF-enforced):
//! * `GET  /` — list `[{id,title,dept_id,dept_name,isenabled,notes,attachment_count,updated}]`.
//! * `GET  /dept-options` — `[{id,name}]` for the form's department dropdown.
//! * `GET  /:id` — one response incl. body + attachment list.
//! * `POST /` (multipart) — create.
//! * `PUT  /:id` (multipart) — edit; `keep_file_ids` retains listed attachments.
//! * `DELETE /:id` — delete (cascades `canned_attachment`).
//! * `POST /mass` — enable / disable / delete.
//!
//! @implements FS-022: canned-response CRUD (create/edit/delete/mass).
//! @implements FS-022.13: attachment upload reuses the seeded upload policy.
//! @implements BS-001: delegated capability gate (admin OR can_manage_premade).

use axum::extract::{FromRequest, Multipart, Path, Request, State};
use axum::Json;
use serde_json::{json, Value};

use ost_core::permission::PERM_CAN_MANAGE_PREMADE;
use ost_core::{log, ApiError, AttachmentSpec, LogType};

use crate::attachments::{drain_multipart, load_upload_policy, validate_attachment, MultipartForm};
use crate::auth::gate::require_admin_or_permission;
use crate::auth::realm::StaffSession;
use crate::auth::StaffCsrf;
use crate::logs::client_ip;
use crate::state::AppState;

/// Gate every canned route: admin OR `can_manage_premade`.
async fn gate(state: &AppState, session: &StaffSession) -> Result<(), ApiError> {
    require_admin_or_permission(state, session, PERM_CAN_MANAGE_PREMADE).await
}

// ---------------------------------------------------------------------------
// GET /api/staff/canned-responses — list.
// ---------------------------------------------------------------------------

/// `GET /api/staff/canned-responses` — every response (premade-gated).
///
/// @implements FS-022: canned-response list with attachment counts.
pub async fn list_canned(
    State(state): State<AppState>,
    session: StaffSession,
) -> Result<Json<Value>, ApiError> {
    gate(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Canned responses are unavailable"))?;

    // (id, title, dept_id, dept_name, isenabled, notes, attachment_count, updated)
    type Row = (
        i32,
        String,
        i32,
        Option<String>,
        bool,
        Option<String>,
        i64,
        Option<String>,
    );
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT cr.canned_id, cr.title, cr.dept_id, d.dept_name, cr.isenabled, \
                COALESCE(cr.notes, '') AS notes, \
                (SELECT COUNT(*) FROM canned_attachment ca WHERE ca.canned_id = cr.canned_id) AS attachment_count, \
                to_char(cr.updated, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated \
         FROM canned_response cr \
         LEFT JOIN department d ON d.dept_id = cr.dept_id \
         ORDER BY cr.title ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Canned list failed"))?;

    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, title, dept_id, dept_name, isenabled, notes, count, updated)| {
            json!({
                "id": id,
                "title": title,
                "dept_id": dept_id,
                "dept_name": dept_name,
                "isenabled": isenabled,
                "notes": notes,
                "attachment_count": count,
                "updated": updated,
            })
        })
        .collect();

    Ok(Json(json!(items)))
}

/// `GET /api/staff/canned-responses/dept-options` — the department options the
/// canned form's dropdown needs (premade-gated, readable without the admin gate).
///
/// @implements FS-022: dept-scope options under the premade gate.
pub async fn dept_options(
    State(state): State<AppState>,
    session: StaffSession,
) -> Result<Json<Value>, ApiError> {
    gate(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Departments are unavailable"))?;

    let rows: Vec<(i32, String)> =
        sqlx::query_as("SELECT dept_id, dept_name FROM department ORDER BY dept_name ASC")
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::internal("Department lookup failed"))?;

    let out: Vec<Value> = rows
        .into_iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();
    Ok(Json(json!(out)))
}

// ---------------------------------------------------------------------------
// GET /api/staff/canned-responses/:id — detail.
// ---------------------------------------------------------------------------

/// `GET /api/staff/canned-responses/:id` — one response with its body +
/// attachment list. 404 when unknown.
///
/// @implements FS-022: canned-response detail.
pub async fn get_canned(
    State(state): State<AppState>,
    session: StaffSession,
    Path(id): Path<i32>,
) -> Result<Json<Value>, ApiError> {
    gate(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Canned responses are unavailable"))?;

    // (id, title, dept_id, response, isenabled, notes)
    type Row = (i32, String, i32, String, bool, Option<String>);
    let row: Option<Row> = sqlx::query_as(
        "SELECT canned_id, title, dept_id, response, isenabled, COALESCE(notes, '') AS notes \
         FROM canned_response WHERE canned_id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal("Canned lookup failed"))?;

    let (id, title, dept_id, response, isenabled, notes) =
        row.ok_or_else(|| ApiError::not_found("Canned response not found"))?;

    // Attachment views: the `id` is the shared attachment_file id (what
    // keep_file_ids references on edit).
    let att_rows: Vec<(i64, String, i64, String)> = sqlx::query_as(
        "SELECT af.id, af.name, af.size, af.mime \
         FROM canned_attachment ca JOIN attachment_file af ON af.id = ca.file_id \
         WHERE ca.canned_id = $1 ORDER BY ca.id ASC",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Attachment lookup failed"))?;

    let attachments: Vec<Value> = att_rows
        .into_iter()
        .map(|(fid, name, size, mime)| json!({ "id": fid, "name": name, "size": size, "mime": mime }))
        .collect();

    Ok(Json(json!({
        "id": id,
        "title": title,
        "dept_id": dept_id,
        "response": response,
        "isenabled": isenabled,
        "notes": notes,
        "attachments": attachments,
    })))
}

// ---------------------------------------------------------------------------
// Shared multipart parsing.
// ---------------------------------------------------------------------------

/// The parsed canned-response write form.
struct CannedForm {
    title: String,
    dept_id: i32,
    response: String,
    isenabled: bool,
    notes: Option<String>,
    attachment: Option<AttachmentSpec>,
    /// Attachment-file ids to retain on edit (comma-separated `keep_file_ids`).
    keep_file_ids: Vec<i64>,
}

/// Parse a truthy multipart text field (`true`/`1`/`yes` ⇒ true).
fn field_bool(form: &MultipartForm, names: &[&str], default: bool) -> bool {
    for n in names {
        if let Some(v) = form.fields.get(*n) {
            let v = v.trim().to_ascii_lowercase();
            return v == "true" || v == "1" || v == "yes" || v == "on";
        }
    }
    default
}

/// Parse the comma-separated `keep_file_ids` (or `keep_file_ids[]`) field into a
/// list of ids. drain_multipart collapses repeated field names, so the reliable
/// wire form is one comma-separated field.
fn parse_keep_ids(form: &MultipartForm) -> Vec<i64> {
    for n in ["keep_file_ids", "keep_file_ids[]", "keepFileIds"] {
        if let Some(v) = form.fields.get(n) {
            return v
                .split(',')
                .filter_map(|s| s.trim().parse::<i64>().ok())
                .collect();
        }
    }
    Vec::new()
}

/// Drain the request body into a [`CannedForm`], validating any attachment part
/// against the upload policy (422 keyed on `attachment` on failure).
async fn parse_canned_form(
    request: Request,
    state: &AppState,
    pool: &sqlx::postgres::PgPool,
) -> Result<CannedForm, ApiError> {
    let multipart = Multipart::from_request(request, state)
        .await
        .map_err(|_| ApiError::validation("Expected a multipart/form-data request"))?;
    let form = drain_multipart(multipart).await?;

    if form.attachment.is_some() {
        let policy = load_upload_policy(pool).await?;
        validate_attachment(&form.attachment, &policy)?;
    }

    let dept_id = form
        .field("dept_id")
        .trim()
        .parse::<i32>()
        .unwrap_or(0);
    let notes = {
        let n = form.field("notes");
        if n.trim().is_empty() {
            None
        } else {
            Some(n.to_string())
        }
    };

    Ok(CannedForm {
        title: form.field("title").trim().to_string(),
        dept_id,
        response: form.field("response").to_string(),
        isenabled: field_bool(&form, &["isenabled", "active"], true),
        notes,
        keep_file_ids: parse_keep_ids(&form),
        attachment: form.attachment,
    })
}

/// Stage an attachment's bytes into the blob store then upsert the
/// `attachment_file` row by content hash, returning its id (inside `tx`).
async fn stage_attachment_file(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    state: &AppState,
    spec: &AttachmentSpec,
) -> Result<i64, ApiError> {
    // Store the bytes once (content-addressed, dedup-idempotent).
    state
        .store
        .put(&spec.bytes)
        .await
        .map_err(|_| ApiError::internal("Could not store the attachment"))?;

    let file_id: i64 = sqlx::query_scalar(
        "INSERT INTO attachment_file (mime, size, hash, name, storage_key) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (hash) DO UPDATE SET name = attachment_file.name RETURNING id",
    )
    .bind(&spec.mime)
    .bind(spec.bytes.len() as i64)
    .bind(spec.hash())
    .bind(&spec.name)
    .bind(spec.storage_key())
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| ApiError::internal("Could not record the attachment"))?;
    Ok(file_id)
}

/// Map a unique-violation on `canned_response_title_key` (23505) to a 422 keyed
/// on `title`.
fn map_canned_write_error(e: sqlx::Error, context: &'static str) -> ApiError {
    if let sqlx::Error::Database(db) = &e {
        if db.code().as_deref() == Some("23505")
            && db.constraint() == Some("canned_response_title_key")
        {
            return ApiError::validation("A canned response with this title already exists")
                .with_field("title", "A canned response with this title already exists");
        }
    }
    tracing::warn!(error = %e, context, "canned write failed");
    ApiError::internal("Could not save the canned response")
}

// ---------------------------------------------------------------------------
// POST /api/staff/canned-responses — create (multipart).
// ---------------------------------------------------------------------------

/// `POST /api/staff/canned-responses` — create (multipart, premade-gated, CSRF).
///
/// @implements FS-022: create (title required + unique) + optional attachment.
pub async fn create_canned(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    gate(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Canned create is unavailable"))?
        .clone();

    let ip = client_ip(request.headers());
    let form = parse_canned_form(request, &state, &pool).await?;

    if form.title.is_empty() {
        return Err(ApiError::validation("Title required").with_field("title", "Title required"));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal("Canned create failed"))?;

    let canned_id: i32 = sqlx::query_scalar(
        "INSERT INTO canned_response (title, dept_id, response, isenabled, notes) \
         VALUES ($1, $2, $3, $4, $5) RETURNING canned_id",
    )
    .bind(&form.title)
    .bind(form.dept_id)
    .bind(&form.response)
    .bind(form.isenabled)
    .bind(form.notes.as_deref())
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| map_canned_write_error(e, "create_canned"))?;

    if let Some(spec) = &form.attachment {
        let file_id = stage_attachment_file(&mut tx, &state, spec).await?;
        sqlx::query(
            "INSERT INTO canned_attachment (canned_id, file_id) VALUES ($1, $2) \
             ON CONFLICT (canned_id, file_id) DO NOTHING",
        )
        .bind(canned_id)
        .bind(file_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("Could not bind the attachment"))?;
    }

    tx.commit()
        .await
        .map_err(|_| ApiError::internal("Canned create failed"))?;

    log(
        &pool,
        LogType::Debug,
        "Canned response created",
        format!(
            "Canned response '{}' (#{canned_id}) created by staff #{}",
            form.title, session.staff_id
        ),
        ip,
    )
    .await;

    Ok(Json(json!({ "id": canned_id })))
}

// ---------------------------------------------------------------------------
// PUT /api/staff/canned-responses/:id — edit (multipart).
// ---------------------------------------------------------------------------

/// `PUT /api/staff/canned-responses/:id` — edit (multipart, premade-gated, CSRF).
///
/// `keep_file_ids` (comma-separated attachment-file ids) retains the listed
/// existing attachments; any not listed are unlinked. A new `attachment` part is
/// staged + linked as well.
///
/// @implements FS-022: edit + attachment retention via keep_file_ids.
pub async fn update_canned(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i32>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    gate(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Canned edit is unavailable"))?
        .clone();

    let exists: Option<i32> =
        sqlx::query_scalar("SELECT canned_id FROM canned_response WHERE canned_id = $1")
            .bind(id)
            .fetch_optional(&pool)
            .await
            .map_err(|_| ApiError::internal("Canned lookup failed"))?;
    if exists.is_none() {
        return Err(ApiError::not_found("Canned response not found"));
    }

    let form = parse_canned_form(request, &state, &pool).await?;

    if form.title.is_empty() {
        return Err(ApiError::validation("Title required").with_field("title", "Title required"));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal("Canned edit failed"))?;

    sqlx::query(
        "UPDATE canned_response SET title = $2, dept_id = $3, response = $4, isenabled = $5, \
                notes = $6, updated = now() WHERE canned_id = $1",
    )
    .bind(id)
    .bind(&form.title)
    .bind(form.dept_id)
    .bind(&form.response)
    .bind(form.isenabled)
    .bind(form.notes.as_deref())
    .execute(&mut *tx)
    .await
    .map_err(|e| map_canned_write_error(e, "update_canned"))?;

    // Retention: unlink every existing binding NOT listed in keep_file_ids.
    if form.keep_file_ids.is_empty() {
        sqlx::query("DELETE FROM canned_attachment WHERE canned_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::internal("Could not update attachments"))?;
    } else {
        sqlx::query(
            "DELETE FROM canned_attachment WHERE canned_id = $1 AND file_id <> ALL($2)",
        )
        .bind(id)
        .bind(&form.keep_file_ids)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("Could not update attachments"))?;
    }

    // A newly-uploaded attachment is staged + linked.
    if let Some(spec) = &form.attachment {
        let file_id = stage_attachment_file(&mut tx, &state, spec).await?;
        sqlx::query(
            "INSERT INTO canned_attachment (canned_id, file_id) VALUES ($1, $2) \
             ON CONFLICT (canned_id, file_id) DO NOTHING",
        )
        .bind(id)
        .bind(file_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("Could not bind the attachment"))?;
    }

    tx.commit()
        .await
        .map_err(|_| ApiError::internal("Canned edit failed"))?;

    Ok(Json(json!({ "id": id })))
}

// ---------------------------------------------------------------------------
// DELETE /api/staff/canned-responses/:id — delete.
// ---------------------------------------------------------------------------

/// `DELETE /api/staff/canned-responses/:id` — delete (premade-gated, CSRF).
/// `canned_attachment` rows cascade with the response.
///
/// @implements FS-022: delete a canned response.
pub async fn delete_canned(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i32>,
    headers: http::HeaderMap,
) -> Result<Json<Value>, ApiError> {
    gate(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Canned delete is unavailable"))?;

    let affected = sqlx::query("DELETE FROM canned_response WHERE canned_id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|_| ApiError::internal("Canned delete failed"))?
        .rows_affected();

    if affected == 0 {
        return Err(ApiError::not_found("Canned response not found"));
    }

    log(
        pool,
        LogType::Debug,
        "Canned response deleted",
        format!("Canned response #{id} deleted by staff #{}", session.staff_id),
        client_ip(&headers),
    )
    .await;

    Ok(Json(json!({ "ok": true })))
}

// ---------------------------------------------------------------------------
// POST /api/staff/canned-responses/mass — enable / disable / delete.
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassCannedRequest {
    pub action: String,
    #[serde(default)]
    pub ids: Vec<i32>,
}

/// `POST /api/staff/canned-responses/mass` — enable / disable / delete
/// (premade-gated, CSRF).
///
/// @implements FS-022: canned-response mass actions.
pub async fn mass_canned(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Json(body): Json<MassCannedRequest>,
) -> Result<Json<Value>, ApiError> {
    gate(&state, &session).await?;
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Mass action is unavailable"))?;

    let action = body.action.trim().to_lowercase();
    if !matches!(action.as_str(), "enable" | "disable" | "delete") {
        return Err(ApiError::validation("Unknown mass action"));
    }
    if body.ids.is_empty() {
        return Err(ApiError::validation("You must select at least one canned response."));
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
            let n = sqlx::query(
                "UPDATE canned_response SET isenabled = true, updated = now() WHERE canned_id = ANY($1)",
            )
            .bind(&ids)
            .execute(pool)
            .await
            .map_err(|_| ApiError::internal("Enable failed"))?
            .rows_affected() as i64;
            (n, "Selected canned responses enabled".to_string())
        }
        "disable" => {
            let n = sqlx::query(
                "UPDATE canned_response SET isenabled = false, updated = now() WHERE canned_id = ANY($1)",
            )
            .bind(&ids)
            .execute(pool)
            .await
            .map_err(|_| ApiError::internal("Disable failed"))?
            .rows_affected() as i64;
            (n, "Selected canned responses disabled".to_string())
        }
        "delete" => {
            let n = sqlx::query("DELETE FROM canned_response WHERE canned_id = ANY($1)")
                .bind(&ids)
                .execute(pool)
                .await
                .map_err(|_| ApiError::internal("Delete failed"))?
                .rows_affected() as i64;
            let msg = if n == requested {
                "Selected canned responses deleted successfully".to_string()
            } else if n == 0 {
                "Unable to delete selected canned responses".to_string()
            } else {
                format!("{n} of {requested} selected canned responses deleted")
            };
            (n, msg)
        }
        _ => unreachable!(),
    };

    Ok(Json(json!({ "affected": affected, "message": message })))
}
