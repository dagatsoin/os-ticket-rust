//! HTTP-layer permission gate helper (TS-M1-A4b).
//!
//! @implements BS-001: enforce a *named* permission on an authenticated staff
//!   session, mapping a denial to the shared 403 error envelope.
//!
//! Thin glue over [`ost_core::session::SessionStore::staff_group_permissions`]
//! and [`ost_core::permission::GroupPermissions::require`]: routes (e.g. C3's
//! staff reply) call [`require_staff_permission`] with the permission name they
//! need, keeping the check generic rather than hardcoded.

use ost_core::permission::PermissionDenied;
use ost_core::ApiError;

use crate::auth::realm::StaffSession;
use crate::state::AppState;

/// Require that the staff session's group grants the **named** permission.
///
/// Returns `Ok(())` when granted; a 403 error envelope otherwise (401 if the
/// account/group can no longer be resolved, e.g. a stale session). Generic over
/// the permission name so later milestones reuse it unchanged.
///
/// @implements BS-001: generic named-permission enforcement at the HTTP layer.
pub async fn require_staff_permission(
    state: &AppState,
    session: &StaffSession,
    permission: &str,
) -> Result<(), ApiError> {
    let store = state
        .sessions
        .as_ref()
        .ok_or_else(|| ApiError::unauthenticated("Authentication required"))?;

    let perms = store
        .staff_group_permissions(session.staff_id)
        .await
        .map_err(|_| ApiError::internal("Permission lookup failed"))?
        .ok_or_else(|| ApiError::unauthenticated("Session expired or invalid"))?;

    perms.require(permission).map_err(|denied| match denied {
        PermissionDenied::GroupDisabled | PermissionDenied::Missing(_) => {
            ApiError::forbidden("You do not have permission to perform this action")
        }
        // An unknown permission name is a programming error; fail closed as 403.
        PermissionDenied::Unknown(_) => {
            ApiError::forbidden("You do not have permission to perform this action")
        }
    })
}

/// Require that the staff session belongs to an **administrator** (`staff.isadmin`).
///
/// This is the per-route gate for the admin configuration panel (FS-032): it is
/// applied inside each admin handler (NOT as blanket middleware), so ordinary
/// staff routes are unaffected. Returns `Ok(())` for an admin; a 401 when the
/// account/group can no longer be resolved (stale session), else a 403.
///
/// The caller has already passed the [`StaffSession`] extractor (so an entirely
/// unauthenticated request is a 401 before this runs); this adds the admin check.
///
/// @implements FS-032.1: administrator-only gate for the settings endpoints.
pub async fn require_admin(state: &AppState, session: &StaffSession) -> Result<(), ApiError> {
    let store = state
        .sessions
        .as_ref()
        .ok_or_else(|| ApiError::unauthenticated("Authentication required"))?;

    let (isadmin, _perms) = store
        .staff_capabilities(session.staff_id)
        .await
        .map_err(|_| ApiError::internal("Permission lookup failed"))?
        .ok_or_else(|| ApiError::unauthenticated("Session expired or invalid"))?;

    if isadmin {
        Ok(())
    } else {
        Err(ApiError::forbidden(
            "Administrator access is required",
        ))
    }
}

/// Require that the session is EITHER an administrator OR carries the named
/// staff permission (TS-M4-H1). Admits an admin unconditionally; otherwise falls
/// back to the generic named-permission gate.
///
/// This is the delegated-management gate: an admin always passes, and a
/// non-admin staff member passes only when their group grants `permission`
/// (e.g. `can_manage_premade` for canned responses, `can_manage_faq` for the
/// FAQ categories reused by EPIC-M4-E). Returns a 403 envelope otherwise.
///
/// @implements FS-022 / BS-001: delegated capability gate (admin OR permission).
pub async fn require_admin_or_permission(
    state: &AppState,
    session: &StaffSession,
    permission: &str,
) -> Result<(), ApiError> {
    // Admin short-circuit: an administrator always passes.
    if require_admin(state, session).await.is_ok() {
        return Ok(());
    }
    // Otherwise the group must carry the named permission.
    require_staff_permission(state, session, permission).await
}
