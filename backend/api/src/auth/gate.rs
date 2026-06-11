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
