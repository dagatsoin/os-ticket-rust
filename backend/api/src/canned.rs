//! Staff canned-response fetch routes (TS-M2-D2).
//!
//! The two routes the reply-box dropdown calls:
//! * `GET /api/staff/tickets/:id/canned` — list the **enabled**, dept-scoped
//!   canned responses for a ticket (BS-022.1 / BS-022.2): `[{id, title}]`.
//! * `GET /api/staff/tickets/:id/canned/:cannedId` — one canned response with
//!   its body **variable-substituted** for THAT ticket (FS-022.14) plus its
//!   attachment list under the shared `attachments` key (§7).
//!
//! A disabled / unknown / out-of-scope canned id ⇒ 404 (no existence leak).
//!
//! Both routes are read-only (no CSRF); the staff realm gate yields 401 without
//! a valid staff session.
//!
//! @implements FS-022.14: canned list + substituted-body detail.
//! @implements BS-022.1 / BS-022.2: enabled + dept-scoped offers; 404 otherwise.
//! @implements ROADMAP §6 / §7: helpdesk_url backs %{url}; shared attachments key.

use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;

use ost_core::canned::{list_offerable, load_offerable_response, load_ticket_vars};
use ost_core::variable::VariableReplacer;
use ost_core::{ApiError, AttachmentView, CannedListItem};

use crate::auth::realm::StaffSession;
use crate::config_keys::read_config;
use crate::state::AppState;

/// Config key holding the helpdesk base URL backing `%{url}` (§6).
pub const CFG_HELPDESK_URL: &str = "helpdesk_url";
/// Fallback helpdesk URL when the config key is unset (matches the seed default).
pub const DEFAULT_HELPDESK_URL: &str = "http://localhost:3702";

/// The substituted canned detail payload (`GET .../canned/:cannedId`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CannedDetail {
    pub id: i32,
    pub title: String,
    /// The body run through the substitution engine for this ticket (§6 `%{url}`).
    pub body: String,
    /// Attachments under the shared §7 key `[{id, name, size, mime}]`.
    pub attachments: Vec<AttachmentView>,
}

/// `GET /api/staff/tickets/:id/canned` — list enabled, dept-scoped canned
/// responses for the ticket.
///
/// 404 when the ticket id is unknown (so the dropdown never lists responses for
/// a non-existent ticket). The list is empty (not an error) when no canned
/// response is in scope.
///
/// @implements BS-022.1 / BS-022.2: enabled + dept-scoped offer list.
pub async fn list_canned(
    State(state): State<AppState>,
    _session: StaffSession,
    Path(id): Path<i64>,
) -> Result<Json<Vec<CannedListItem>>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Canned responses are unavailable"))?;

    // The ticket must exist (and gives us its department for scoping).
    let vars = load_ticket_vars(pool, id)
        .await
        .map_err(|_| ApiError::internal("Ticket lookup failed"))?
        .ok_or_else(|| ApiError::not_found("Ticket not found"))?;

    let items = list_offerable(pool, vars.dept_id)
        .await
        .map_err(|_| ApiError::internal("Canned response lookup failed"))?;
    Ok(Json(items))
}

/// `GET /api/staff/tickets/:id/canned/:cannedId` — one canned response with its
/// body substituted for this ticket + its §7 attachment list.
///
/// 404 when the ticket id is unknown OR the canned id is disabled / unknown /
/// out of the ticket's department scope (BS-022.2 — no existence leak).
///
/// @implements FS-022.14: substituted-body detail + shared attachments key (§7).
pub async fn get_canned(
    State(state): State<AppState>,
    _session: StaffSession,
    Path((id, canned_id)): Path<(i64, i32)>,
) -> Result<Json<CannedDetail>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Canned responses are unavailable"))?;

    let vars = load_ticket_vars(pool, id)
        .await
        .map_err(|_| ApiError::internal("Ticket lookup failed"))?
        .ok_or_else(|| ApiError::not_found("Ticket not found"))?;

    // Disabled / unknown / out-of-scope canned id ⇒ 404 (BS-022.2).
    let canned = load_offerable_response(pool, canned_id, vars.dept_id)
        .await
        .map_err(|_| ApiError::internal("Canned response lookup failed"))?
        .ok_or_else(|| ApiError::not_found("Canned response not found"))?;

    // Substitute the body against THIS ticket + the helpdesk base URL (§6).
    let base_url = read_config(pool, CFG_HELPDESK_URL)
        .await?
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_HELPDESK_URL.to_string());
    let replacer = VariableReplacer::new(vars.to_context(), base_url);
    let body = replacer.render(&canned.body);

    Ok(Json(CannedDetail {
        id: canned.id,
        title: canned.title,
        body,
        attachments: canned.attachments,
    }))
}
