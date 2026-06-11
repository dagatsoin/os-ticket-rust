//! Ticket-event → outbound email wiring (TS-M2-E3).
//!
//! Shared helpers that render a packaged-default template for a ticket and
//! dispatch it through the active mailer transport. Both fire **after the DB
//! commit** (M1 convention — no mail for a failed write) and **always send** in
//! M2 (ROADMAP §12; per-department flags arrive in M5), subject only to the
//! FS-011.12 loop-suppression guard on the autoresponse.
//!
//! `%{url}` is backed by the seeded `helpdesk_url` config key (§6); when the key
//! is unset the [`crate::canned::DEFAULT_HELPDESK_URL`] fallback is used.
//!
//! @implements FS-011.12: new-ticket autoresponse on create (always-send, §12).
//! @implements FS-021.3: staff-reply notification on reply (always-send, §12).
//! @implements ROADMAP §6: `%{url}` from the `helpdesk_url` config key.

use sqlx::postgres::PgPool;

use ost_core::canned::load_ticket_vars;
use ost_core::email::{
    is_loop_suppressed_recipient, send_autoreply, send_notice, EmailTemplate,
    NEW_TICKET_AUTORESPONSE, STAFF_REPLY_NOTIFICATION,
};
use ost_core::variable::VariableReplacer;
use ost_core::OutboundMail;

use crate::canned::{CFG_HELPDESK_URL, DEFAULT_HELPDESK_URL};
use crate::config_keys::read_config;
use crate::state::AppState;

/// Read the helpdesk base URL backing `%{url}` (§6), falling back to the seed
/// default when the config key is unset/blank.
async fn helpdesk_url(pool: &PgPool) -> String {
    read_config(pool, CFG_HELPDESK_URL)
        .await
        .ok()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_HELPDESK_URL.to_string())
}

/// Render `template` for `ticket_id` into an [`OutboundMail`] to the requester.
///
/// `None` when the ticket can't be loaded (so the caller silently skips mail
/// rather than failing the already-committed request).
async fn render_for_ticket(
    pool: &PgPool,
    ticket_id: i64,
    template: EmailTemplate,
) -> Option<OutboundMail> {
    let vars = load_ticket_vars(pool, ticket_id).await.ok().flatten()?;
    let base_url = helpdesk_url(pool).await;
    let replacer = VariableReplacer::new(vars.to_context(), base_url);
    let (subject, body) = template.render(&replacer);
    Some(OutboundMail::new(vars.email, subject, body))
}

/// Send the new-ticket **autoresponse** to the requester after a successful
/// create (FS-011.12). Always-send in M2 (§12), suppressed only for a
/// daemon/postmaster requester address (loop prevention). Failures are logged,
/// never surfaced — the ticket is already committed.
///
/// @implements FS-011.12: autoresponse on create (auto-reply wrapper, §12).
pub async fn send_new_ticket_autoresponse(state: &AppState, pool: &PgPool, ticket_id: i64) {
    let Some(mail) = render_for_ticket(pool, ticket_id, NEW_TICKET_AUTORESPONSE).await else {
        tracing::warn!(ticket_id, "autoresponse skipped: ticket vars unavailable");
        return;
    };
    if is_loop_suppressed_recipient(&mail.to) {
        tracing::info!(to = %mail.to, "autoresponse suppressed (loop-prevention)");
        return;
    }
    if let Err(err) = send_autoreply(state.mailer.active.as_ref(), mail).await {
        tracing::warn!(error = %err, "new-ticket autoresponse send failed");
    }
}

/// Send the staff-reply **notification** to the requester after a committed
/// reply (FS-021.3). Always-send in M2 (§12), via the notice wrapper.
///
/// @implements FS-021.3: notification on reply (notice wrapper, §12).
pub async fn send_reply_notification(state: &AppState, pool: &PgPool, ticket_id: i64) {
    let Some(mail) = render_for_ticket(pool, ticket_id, STAFF_REPLY_NOTIFICATION).await else {
        tracing::warn!(ticket_id, "reply notification skipped: ticket vars unavailable");
        return;
    };
    if let Err(err) = send_notice(state.mailer.active.as_ref(), mail).await {
        tracing::warn!(error = %err, "reply notification send failed");
    }
}
