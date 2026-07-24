//! Admin help-topic management (TS-M4-C5) — FS-030.15/.16/.17.
//!
//! Admin-gated CRUD over the `help_topic` table with routing fields (department,
//! priority, SLA, auto-assign staff-XOR-team, thank-you page), one-level parent
//! nesting enforced write-side, and a delete that promotes children to top-level.
//!
//! Route shapes (all under `/api/staff/admin`, admin-gated per-handler; mutating
//! routes CSRF-enforced):
//! * `GET  /help-topics` — paginated list.
//! * `GET  /help-topics/form-options` — select sources (pages = thank-you only;
//!   parent_topics = top-level only).
//! * `GET  /help-topics/:id` — one topic (404 when absent).
//! * `POST /help-topics` — create.
//! * `PUT  /help-topics/:id` — edit.
//! * `POST /help-topics/mass` — enable / disable / delete.
//! * `DELETE /help-topics/:id` — delete + promote children + clear references.
//!
//! @implements FS-030.15: help-topic list + create (no no-op guard).
//! @implements FS-030.16: help-topic edit + routing fields.
//! @implements FS-030.17: help-topic delete promotes children.
//! @implements BS-030-18: text required ≥5 chars, unique-within-parent.
//! @implements BS-030-19: department + priority required.
//! @implements BS-030-20: ONE-LEVEL nesting enforced write-side.
//! @implements BS-030-22: auto-assign staff XOR team (`s`/`t` encoding).
//! @implements BS-030-23: SLA 0 = no topic SLA; empty page = system default.
//! @implements BS-030-26: delete promotes children + clears ticket.topic_id.

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
        .ok_or_else(|| ApiError::internal("Help topics are unavailable"))
}

// ---------------------------------------------------------------------------
// GET /api/staff/admin/help-topics — paginated list.
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
pub struct TopicListParams {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub order: Option<String>,
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub page_size: Option<i64>,
}

/// Upper bound on the `page` query parameter. Clamping `page` (like `page_size`)
/// keeps the `(page - 1) * page_size` offset well within `i64` so an oversized
/// `page` (e.g. `i64::MAX`) can never overflow into a 500 (WARN-4a). At the
/// 200-row `page_size` ceiling this still addresses 200 million rows.
const MAX_PAGE: i64 = 1_000_000;

/// Resolve the validated `(page, page_size, offset)` triple from the raw query
/// params, clamping BOTH `page` (`1..=MAX_PAGE`) and `page_size` (`1..=200`) and
/// computing the offset with saturating arithmetic (WARN-4a — no overflow).
fn clamp_page(page: Option<i64>, page_size: Option<i64>) -> (i64, i64, i64) {
    let page = page.unwrap_or(1).clamp(1, MAX_PAGE);
    let page_size = page_size.unwrap_or(25).clamp(1, 200);
    let offset = (page - 1).saturating_mul(page_size);
    (page, page_size, offset)
}

fn topic_sort_expr(sort: Option<&str>) -> &'static str {
    match sort.unwrap_or("topic") {
        "status" | "isactive" => "h.isactive",
        "type" | "ispublic" => "h.ispublic",
        "dept" => "dept_name",
        "priority" => "h.priority_id",
        "updated" => "h.updated",
        "created" => "h.created",
        _ => "h.topic",
    }
}

/// `GET /api/staff/admin/help-topics` — paginated list (admin only). `page_size`
/// defaults to 25.
///
/// @implements FS-030.15: help-topic list (paginated, sort incl updated).
pub async fn list_topics(
    State(state): State<AppState>,
    session: StaffSession,
    Query(params): Query<TopicListParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    let order = match params.order.as_deref().map(|s| s.to_uppercase()) {
        Some(ref o) if o == "DESC" => "DESC",
        _ => "ASC",
    };
    let order_by = format!("{} {order}, h.topic_id ASC", topic_sort_expr(params.sort.as_deref()));

    let (page, page_size, offset) = clamp_page(params.page, params.page_size);

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM help_topic")
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal("Help-topic count failed"))?;

    let sql = format!(
        "SELECT h.topic_id, h.topic, h.topic_pid, COALESCE(p.topic, '') AS parent_topic, \
                h.isactive, h.ispublic, h.priority_id, COALESCE(pr.priority_desc, '') AS priority, \
                h.dept_id, COALESCE(d.dept_name, '') AS dept_name, h.sla_id, h.staff_id, h.team_id, \
                h.page_id, to_char(h.updated, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated \
         FROM help_topic h \
         LEFT JOIN help_topic p ON p.topic_id = h.topic_pid \
         LEFT JOIN priority pr ON pr.priority_id = h.priority_id \
         LEFT JOIN department d ON d.dept_id = h.dept_id \
         ORDER BY {order_by} LIMIT {page_size} OFFSET {offset}"
    );

    // (topic_id, topic, topic_pid, parent_topic, isactive, ispublic, priority_id, priority,
    //  dept_id, dept_name, sla_id, staff_id, team_id, page_id, updated)
    type Row = (
        i32,
        String,
        i32,
        String,
        bool,
        bool,
        i32,
        String,
        Option<i32>,
        String,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        i32,
        Option<String>,
    );
    let rows: Vec<Row> = sqlx::query_as(&sql)
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::internal("Help-topic list failed"))?;

    let items: Vec<Value> = rows
        .into_iter()
        .map(
            |(topic_id, topic, topic_pid, parent_topic, isactive, ispublic, priority_id, priority, dept_id, dept_name, sla_id, staff_id, team_id, page_id, updated)| {
                json!({
                    "topic_id": topic_id,
                    "topic": topic,
                    "topic_pid": topic_pid,
                    "parent_topic": parent_topic,
                    "isactive": isactive,
                    "ispublic": ispublic,
                    "priority_id": priority_id,
                    "priority": priority,
                    "dept_id": dept_id,
                    "dept_name": dept_name,
                    "sla_id": sla_id,
                    "staff_id": staff_id,
                    "team_id": team_id,
                    "page_id": page_id,
                    "updated": updated,
                })
            },
        )
        .collect();

    Ok(Json(json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    })))
}

// ---------------------------------------------------------------------------
// GET /api/staff/admin/help-topics/form-options — select sources.
// ---------------------------------------------------------------------------

/// `GET /api/staff/admin/help-topics/form-options` — the select sources for the
/// topic form. `pages` is filtered to thank-you pages only; `parent_topics` to
/// top-level topics (`topic_pid = 0`) so the picker can never offer a child.
///
/// @implements FS-030.16: help-topic form option sources.
/// @implements BS-030-20: parent picker offers top-level topics only.
/// @implements BS-030-23: thank-you page select source.
pub async fn form_options(
    State(state): State<AppState>,
    session: StaffSession,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    type NameRow = (i32, String);

    // priorities: [{id, name}] ranked most-urgent first.
    let priority_rows: Vec<NameRow> = sqlx::query_as(
        "SELECT priority_id, priority_desc FROM priority ORDER BY urgency DESC, priority_id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Priority lookup failed"))?;
    let priorities: Vec<Value> = priority_rows
        .into_iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();

    let dept_rows: Vec<NameRow> =
        sqlx::query_as("SELECT dept_id, dept_name FROM department ORDER BY dept_name ASC")
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::internal("Department lookup failed"))?;
    let departments: Vec<Value> = dept_rows
        .into_iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();

    let sla_rows: Vec<NameRow> =
        sqlx::query_as("SELECT id, name FROM sla WHERE isactive = true ORDER BY name ASC")
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::internal("SLA lookup failed"))?;
    let sla: Vec<Value> = sla_rows
        .into_iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();

    // pages: thank-you only.
    let page_rows: Vec<NameRow> = sqlx::query_as(
        "SELECT id, name FROM page WHERE type = 'thank-you' AND isactive = true ORDER BY name ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Page lookup failed"))?;
    let pages: Vec<Value> = page_rows
        .into_iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();

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

    let team_rows: Vec<NameRow> =
        sqlx::query_as("SELECT team_id, name FROM team WHERE isenabled = true ORDER BY name ASC")
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::internal("Team lookup failed"))?;
    let teams: Vec<Value> = team_rows
        .into_iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();

    // parent_topics: top-level only (topic_pid = 0).
    let parent_rows: Vec<NameRow> = sqlx::query_as(
        "SELECT topic_id, topic FROM help_topic WHERE topic_pid = 0 ORDER BY topic ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Parent-topic lookup failed"))?;
    let parent_topics: Vec<Value> = parent_rows
        .into_iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();

    Ok(Json(json!({
        "priorities": priorities,
        "departments": departments,
        "sla": sla,
        "pages": pages,
        "staff": staff,
        "teams": teams,
        "parent_topics": parent_topics,
    })))
}

// ---------------------------------------------------------------------------
// GET /api/staff/admin/help-topics/:id — detail.
// ---------------------------------------------------------------------------

/// Encode a topic's auto-assignee to the form's `s<id>` / `t<id>` string (or "").
fn encode_assign_to(staff_id: Option<i32>, team_id: Option<i32>) -> String {
    if let Some(s) = staff_id.filter(|&v| v > 0) {
        format!("s{s}")
    } else if let Some(t) = team_id.filter(|&v| v > 0) {
        format!("t{t}")
    } else {
        String::new()
    }
}

/// `GET /api/staff/admin/help-topics/:id` — one topic; 404 when absent.
///
/// @implements FS-030.16: help-topic detail.
pub async fn get_topic(
    State(state): State<AppState>,
    session: StaffSession,
    Path(id): Path<i32>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    // (topic_id, topic, topic_pid, isactive, ispublic, noautoresp, priority_id,
    //  dept_id, staff_id, team_id, sla_id, page_id, notes, updated)
    type Row = (
        i32,
        String,
        i32,
        bool,
        bool,
        bool,
        i32,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        i32,
        Option<String>,
        Option<String>,
    );
    let row: Option<Row> = sqlx::query_as(
        "SELECT topic_id, topic, topic_pid, isactive, ispublic, noautoresp, priority_id, \
                dept_id, staff_id, team_id, sla_id, page_id, notes, \
                to_char(updated, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated \
         FROM help_topic WHERE topic_id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal("Help-topic lookup failed"))?;

    let row = row.ok_or_else(|| ApiError::not_found("Help topic not found"))?;

    Ok(Json(json!({
        "topic_id": row.0,
        "topic": row.1,
        "topic_pid": row.2,
        "isactive": row.3,
        "ispublic": row.4,
        "noautoresp": row.5,
        "priority_id": row.6,
        "dept_id": row.7,
        "staff_id": row.8,
        "team_id": row.9,
        "sla_id": row.10,
        "page_id": row.11,
        "assign_to": encode_assign_to(row.8, row.9),
        "notes": row.12.unwrap_or_default(),
        "updated": row.13,
    })))
}

// ---------------------------------------------------------------------------
// POST / PUT — create / edit.
// ---------------------------------------------------------------------------

/// Create/edit help-topic request body (camelCase).
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicWriteRequest {
    #[serde(default)]
    pub topic: String,
    #[serde(default = "default_true")]
    pub isactive: bool,
    #[serde(default = "default_true")]
    pub ispublic: bool,
    #[serde(default)]
    pub dept_id: Option<i32>,
    #[serde(default)]
    pub priority_id: Option<i32>,
    #[serde(default)]
    pub sla_id: Option<i32>,
    #[serde(default)]
    pub parent_id: Option<i32>,
    /// Auto-assignee encoded as `s<id>` (staff) / `t<id>` (team) / empty (neither).
    #[serde(default)]
    pub assign_to: Option<String>,
    #[serde(default)]
    pub page_id: Option<i32>,
    #[serde(default)]
    pub noautoresp: bool,
    #[serde(default)]
    pub notes: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Validated + resolved help-topic write fields.
#[derive(Debug)]
struct TopicFields {
    topic: String,
    topic_pid: i32,
    dept_id: i32,
    priority_id: i32,
    sla_id: Option<i32>,
    staff_id: Option<i32>,
    team_id: Option<i32>,
    page_id: i32,
}

/// Decode the `assign_to` string to a (staff_id, team_id) pair — mutually
/// exclusive (BS-030-22). A leading `s` → staff; `t` → team; anything else (or
/// empty) → both NULL.
fn decode_assign_to(assign_to: Option<&str>) -> (Option<i32>, Option<i32>) {
    let Some(raw) = assign_to.map(str::trim).filter(|s| !s.is_empty()) else {
        return (None, None);
    };
    let (tag, rest) = raw.split_at(1);
    let n = rest.parse::<i32>().ok().filter(|&v| v > 0);
    match (tag, n) {
        ("s" | "S", Some(id)) => (Some(id), None),
        ("t" | "T", Some(id)) => (None, Some(id)),
        _ => (None, None),
    }
}

/// Validate + resolve the topic fields (BS-030-18/19/22/23). `parent_ok` reports
/// whether a supplied non-zero parent is itself top-level (BS-030-20 one-level).
fn validate_topic_fields(
    body: &TopicWriteRequest,
    parent_is_top_level: Option<bool>,
) -> Result<TopicFields, ApiError> {
    let mut err = ApiError::validation("Please correct the errors below");
    let topic = body.topic.trim().to_string();
    if topic.chars().count() < 5 {
        err = err.with_field("topic", "5 chars minimum");
    }
    let dept_id = body.dept_id.filter(|&v| v > 0);
    if dept_id.is_none() {
        err = err.with_field("deptId", "You must select a department");
    }
    let priority_id = body.priority_id.filter(|&v| v > 0);
    if priority_id.is_none() {
        err = err.with_field("priorityId", "You must select a priority");
    }
    // BS-030-20: one-level nesting — a supplied parent must be top-level.
    let parent = body.parent_id.filter(|&v| v > 0);
    if parent.is_some() && parent_is_top_level == Some(false) {
        err = err.with_field("parentId", "Parent topic must be a top-level topic");
    }
    if !err.fields.is_empty() {
        return Err(err);
    }

    let (staff_id, team_id) = decode_assign_to(body.assign_to.as_deref());

    Ok(TopicFields {
        topic,
        topic_pid: parent.unwrap_or(0),
        dept_id: dept_id.unwrap(),
        priority_id: priority_id.unwrap(),
        sla_id: body.sla_id.filter(|&v| v > 0),
        staff_id,
        team_id,
        page_id: body.page_id.filter(|&v| v > 0).unwrap_or(0),
    })
}

/// Look up whether a candidate parent topic exists and is top-level. `Ok(None)`
/// means no parent was supplied (top-level topic). `Some(bool)` = is-top-level.
async fn parent_top_level(
    pool: &sqlx::postgres::PgPool,
    parent_id: Option<i32>,
) -> Result<Option<bool>, ApiError> {
    match parent_id.filter(|&v| v > 0) {
        None => Ok(None),
        Some(pid) => {
            let pid_of: Option<i32> =
                sqlx::query_scalar("SELECT topic_pid FROM help_topic WHERE topic_id = $1")
                    .bind(pid)
                    .fetch_optional(pool)
                    .await
                    .map_err(|_| ApiError::internal("Parent lookup failed"))?;
            // Absent parent → treat as not-top-level (rejected write-side).
            Ok(Some(pid_of == Some(0)))
        }
    }
}

/// Whether `id` already has at least one child topic (a row with
/// `topic_pid = id`). Used to forbid nesting a topic that is itself a parent,
/// which would otherwise create a three-level chain (WARN-4b / KL-030-02).
async fn has_children(pool: &sqlx::postgres::PgPool, id: i32) -> Result<bool, ApiError> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM help_topic WHERE topic_pid = $1)")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal("Child lookup failed"))
}

/// Map a unique-violation on `help_topic_topic_pid_key` (23505) to a 422 keyed on
/// `topic`.
fn map_topic_write_error(e: sqlx::Error, context: &'static str) -> ApiError {
    if let sqlx::Error::Database(db) = &e {
        if db.code().as_deref() == Some("23505")
            && db.constraint() == Some("help_topic_topic_pid_key")
        {
            return ApiError::validation("A topic with this name already exists")
                .with_field("topic", "A topic with this name already exists");
        }
    }
    tracing::warn!(error = %e, context, "help-topic write failed");
    ApiError::internal("Could not save the help topic")
}

/// `POST /api/staff/admin/help-topics` — create (admin + CSRF).
///
/// @implements FS-030.15: help-topic create.
/// @implements BS-030-20: one-level nesting enforced write-side.
pub async fn create_topic(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Json(body): Json<TopicWriteRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    let parent_flag = parent_top_level(pool, body.parent_id).await?;
    let f = validate_topic_fields(&body, parent_flag)?;

    let id: i32 = sqlx::query_scalar(
        "INSERT INTO help_topic \
            (topic, topic_pid, isactive, ispublic, noautoresp, priority_id, dept_id, \
             staff_id, team_id, sla_id, page_id, notes) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING topic_id",
    )
    .bind(&f.topic)
    .bind(f.topic_pid)
    .bind(body.isactive)
    .bind(body.ispublic)
    .bind(body.noautoresp)
    .bind(f.priority_id)
    .bind(f.dept_id)
    .bind(f.staff_id)
    .bind(f.team_id)
    .bind(f.sla_id)
    .bind(f.page_id)
    .bind(body.notes.as_deref())
    .fetch_one(pool)
    .await
    .map_err(|e| map_topic_write_error(e, "create_topic"))?;

    Ok(Json(json!({ "id": id })))
}

/// `PUT /api/staff/admin/help-topics/:id` — edit (admin + CSRF). No no-op guard —
/// a re-save with no changes succeeds (FS-030.15).
///
/// @implements FS-030.16: help-topic edit + routing fields.
/// @implements BS-030-22: auto-assign staff XOR team.
pub async fn update_topic(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i32>,
    Json(body): Json<TopicWriteRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    let exists: Option<i32> = sqlx::query_scalar("SELECT topic_id FROM help_topic WHERE topic_id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("Help-topic lookup failed"))?;
    if exists.is_none() {
        return Err(ApiError::not_found("Help topic not found"));
    }

    let parent_flag = parent_top_level(pool, body.parent_id).await?;
    let f = validate_topic_fields(&body, parent_flag)?;
    // A topic may not be its own parent.
    if f.topic_pid == id {
        return Err(ApiError::validation("A topic cannot be its own parent")
            .with_field("parentId", "A topic cannot be its own parent"));
    }
    // BS-030-20 / KL-030-02 (WARN-4b): strict one-level nesting. A topic that
    // already HAS children may not itself be given a parent — doing so would
    // form a three-level chain (child → this → new-parent).
    if f.topic_pid != 0 && has_children(pool, id).await? {
        return Err(ApiError::validation(
            "A topic with sub-topics cannot itself become a sub-topic",
        )
        .with_field(
            "parentId",
            "This topic has sub-topics and cannot be nested under another topic",
        ));
    }

    sqlx::query(
        "UPDATE help_topic SET topic = $2, topic_pid = $3, isactive = $4, ispublic = $5, \
                noautoresp = $6, priority_id = $7, dept_id = $8, staff_id = $9, team_id = $10, \
                sla_id = $11, page_id = $12, notes = $13, updated = now() \
         WHERE topic_id = $1",
    )
    .bind(id)
    .bind(&f.topic)
    .bind(f.topic_pid)
    .bind(body.isactive)
    .bind(body.ispublic)
    .bind(body.noautoresp)
    .bind(f.priority_id)
    .bind(f.dept_id)
    .bind(f.staff_id)
    .bind(f.team_id)
    .bind(f.sla_id)
    .bind(f.page_id)
    .bind(body.notes.as_deref())
    .execute(pool)
    .await
    .map_err(|e| map_topic_write_error(e, "update_topic"))?;

    Ok(Json(json!({ "id": id })))
}

// ---------------------------------------------------------------------------
// DELETE /api/staff/admin/help-topics/:id — delete + promote children.
// ---------------------------------------------------------------------------

/// Promote a topic's children to top-level, clear ticket references, then delete
/// it — all in one transaction. The FAQ-link clear is defined-but-noop (no
/// `faq_topic` table yet).
async fn delete_topic_promote(pool: &sqlx::postgres::PgPool, id: i32) -> Result<(), ApiError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal("Help-topic delete failed"))?;

    sqlx::query("UPDATE help_topic SET topic_pid = 0, updated = now() WHERE topic_pid = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("Help-topic delete failed (child promote)"))?;
    sqlx::query("UPDATE ticket SET topic_id = 0, updated = now() WHERE topic_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("Help-topic delete failed (ticket clear)"))?;
    // FAQ-link clear: defined-but-noop (no faq_topic table yet — M7).
    sqlx::query("DELETE FROM help_topic WHERE topic_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal("Help-topic delete failed"))?;

    tx.commit()
        .await
        .map_err(|_| ApiError::internal("Help-topic delete failed"))?;
    Ok(())
}

/// `DELETE /api/staff/admin/help-topics/:id` — delete + promote children.
///
/// @implements FS-030.17: help-topic delete.
/// @implements BS-030-26: promote children + clear ticket.topic_id.
pub async fn delete_topic(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Path(id): Path<i32>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    let exists: Option<i32> = sqlx::query_scalar("SELECT topic_id FROM help_topic WHERE topic_id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("Help-topic lookup failed"))?;
    if exists.is_none() {
        return Err(ApiError::not_found("Help topic not found"));
    }

    delete_topic_promote(pool, id).await?;

    Ok(Json(json!({ "affected": 1, "message": "Help topic deleted" })))
}

// ---------------------------------------------------------------------------
// POST /api/staff/admin/help-topics/mass — enable / disable / delete.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassTopicRequest {
    pub action: String,
    #[serde(default)]
    pub ids: Vec<i32>,
}

/// `POST /api/staff/admin/help-topics/mass` — enable / disable / delete.
///
/// @implements FS-030.15: help-topic mass actions.
/// @implements BS-030-26: mass delete promotes each topic's children.
pub async fn mass_topics(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
    Json(body): Json<MassTopicRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &session).await?;
    let pool = pool(&state)?;

    let action = body.action.trim().to_lowercase();
    if !matches!(action.as_str(), "enable" | "disable" | "delete") {
        return Err(ApiError::validation("Unknown mass action"));
    }
    if body.ids.is_empty() {
        return Err(ApiError::validation("You must select at least one topic."));
    }
    let ids: Vec<i32> = {
        let mut v = body.ids.clone();
        v.sort_unstable();
        v.dedup();
        v
    };

    let (affected, message) = match action.as_str() {
        "enable" => {
            let n = sqlx::query("UPDATE help_topic SET isactive = true, updated = now() WHERE topic_id = ANY($1)")
                .bind(&ids)
                .execute(pool)
                .await
                .map_err(|_| ApiError::internal("Enable failed"))?
                .rows_affected() as i64;
            (n, "Selected topics enabled".to_string())
        }
        "disable" => {
            let n = sqlx::query("UPDATE help_topic SET isactive = false, updated = now() WHERE topic_id = ANY($1)")
                .bind(&ids)
                .execute(pool)
                .await
                .map_err(|_| ApiError::internal("Disable failed"))?
                .rows_affected() as i64;
            (n, "Selected topics disabled".to_string())
        }
        "delete" => {
            let mut deleted = 0i64;
            for &id in &ids {
                delete_topic_promote(pool, id).await?;
                deleted += 1;
            }
            (deleted, "Selected topics deleted".to_string())
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
        assert_eq!(topic_sort_expr(None), "h.topic");
        assert_eq!(topic_sort_expr(Some("updated")), "h.updated");
        assert_eq!(topic_sort_expr(Some("dept")), "dept_name");
    }

    #[test]
    fn clamp_page_bounds_and_never_overflows() {
        // Defaults.
        assert_eq!(clamp_page(None, None), (1, 25, 0));
        // Normal paging.
        assert_eq!(clamp_page(Some(2), Some(10)), (2, 10, 10));
        // page_size clamped to 1..=200.
        assert_eq!(clamp_page(Some(1), Some(9999)), (1, 200, 0));
        assert_eq!(clamp_page(Some(1), Some(0)), (1, 1, 0));
        // page floored at 1.
        assert_eq!(clamp_page(Some(0), Some(25)), (1, 25, 0));
        assert_eq!(clamp_page(Some(-5), Some(25)), (1, 25, 0));
        // WARN-4a: a hostile huge page cannot overflow the i64 offset multiply.
        let (page, page_size, offset) = clamp_page(Some(i64::MAX), Some(200));
        assert_eq!(page, MAX_PAGE);
        assert_eq!(page_size, 200);
        assert_eq!(offset, (MAX_PAGE - 1) * 200);
        assert!(offset > 0, "offset stays positive (no wrap)");
    }

    #[test]
    fn decode_assign_to_variants() {
        assert_eq!(decode_assign_to(Some("s5")), (Some(5), None));
        assert_eq!(decode_assign_to(Some("t3")), (None, Some(3)));
        assert_eq!(decode_assign_to(Some("")), (None, None));
        assert_eq!(decode_assign_to(None), (None, None));
        assert_eq!(decode_assign_to(Some("x9")), (None, None));
        assert_eq!(decode_assign_to(Some("s0")), (None, None));
    }

    #[test]
    fn encode_assign_to_variants() {
        assert_eq!(encode_assign_to(Some(5), None), "s5");
        assert_eq!(encode_assign_to(None, Some(3)), "t3");
        assert_eq!(encode_assign_to(None, None), "");
        // Staff wins when both present (mutually exclusive on write anyway).
        assert_eq!(encode_assign_to(Some(1), Some(2)), "s1");
    }

    #[test]
    fn validate_requires_topic_dept_priority() {
        let err = validate_topic_fields(&TopicWriteRequest::default(), None).unwrap_err();
        assert!(err.fields.contains_key("topic"));
        assert!(err.fields.contains_key("deptId"));
        assert!(err.fields.contains_key("priorityId"));
    }

    #[test]
    fn validate_rejects_short_topic() {
        let body = TopicWriteRequest {
            topic: "Abcd".into(),
            dept_id: Some(1),
            priority_id: Some(2),
            ..Default::default()
        };
        assert!(validate_topic_fields(&body, None).unwrap_err().fields.contains_key("topic"));
    }

    #[test]
    fn validate_rejects_non_top_level_parent() {
        let body = TopicWriteRequest {
            topic: "Refunds".into(),
            dept_id: Some(1),
            priority_id: Some(2),
            parent_id: Some(9),
            ..Default::default()
        };
        // Parent is NOT top-level → rejected.
        assert!(validate_topic_fields(&body, Some(false)).unwrap_err().fields.contains_key("parentId"));
        // Parent IS top-level → accepted.
        assert!(validate_topic_fields(&body, Some(true)).is_ok());
    }

    #[test]
    fn validate_resolves_fields() {
        let body = TopicWriteRequest {
            topic: "  Password reset  ".into(),
            dept_id: Some(4),
            priority_id: Some(2),
            sla_id: Some(0),
            assign_to: Some("t7".into()),
            page_id: Some(0),
            parent_id: Some(3),
            ..Default::default()
        };
        let f = validate_topic_fields(&body, Some(true)).unwrap();
        assert_eq!(f.topic, "Password reset");
        assert_eq!(f.topic_pid, 3);
        assert_eq!(f.sla_id, None); // 0 → no topic SLA.
        assert_eq!(f.page_id, 0); // 0 → system default.
        assert_eq!(f.team_id, Some(7));
        assert_eq!(f.staff_id, None);
    }
}
