//! Generic named-permission gate for the staff realm (TS-M1-A4b).
//!
//! @implements BS-001: permission enforcement — a staff member may only perform
//!   an action their permission **group** grants.
//!
//! Deliberately *generic*: the gate checks a **named** permission string against
//! the flags carried by the staff member's group, rather than hardcoding a
//! single "can reply" check. C3 (staff reply) and later milestones reuse it by
//! passing a different permission name (`can_create_tickets`, `can_post_reply`,
//! …). New permissions are added to [`GroupPermissions`] as the schema grows.

/// Named permissions the M1 group model exposes. These map 1:1 to the boolean
/// flag columns on the `groups` table; later milestones extend the set.
pub const PERM_CAN_CREATE_TICKETS: &str = "can_create_tickets";
pub const PERM_CAN_POST_REPLY: &str = "can_post_reply";
/// @implements BS-021.1: close/reopen permission (FS-021.11, FS-021.12).
pub const PERM_CAN_CLOSE_TICKETS: &str = "can_close_tickets";
/// @implements BS-021.1: assign/claim permission (FS-021.7, FS-021.8).
pub const PERM_CAN_ASSIGN_TICKETS: &str = "can_assign_tickets";
/// @implements BS-021.1: transfer permission (FS-021.10).
pub const PERM_CAN_TRANSFER_TICKETS: &str = "can_transfer_tickets";
/// @implements BS-021.1: delete permission (FS-021.19).
pub const PERM_CAN_DELETE_TICKETS: &str = "can_delete_tickets";
/// @implements BS-021.1: edit permission (FS-021.15).
pub const PERM_CAN_EDIT_TICKETS: &str = "can_edit_tickets";
/// @implements BS-021.1: mass manage permission (FS-021.21).
pub const PERM_CAN_MANAGE_TICKETS: &str = "can_manage_tickets";
/// @implements FS-032.1: FAQ / knowledgebase management permission (M4).
pub const PERM_CAN_MANAGE_FAQ: &str = "can_manage_faq";
/// @implements FS-032.1: canned/premade-response management permission (M4).
pub const PERM_CAN_MANAGE_PREMADE: &str = "can_manage_premade";
/// @implements FS-032.1: ban-list (banned emails) management permission (M4).
pub const PERM_CAN_BAN_EMAILS: &str = "can_ban_emails";
/// @implements FS-032.1: view staff statistics/dashboard permission (M4).
pub const PERM_CAN_VIEW_STAFF_STATS: &str = "can_view_staff_stats";

/// The permission flags a staff member's group grants, loaded from the `groups`
/// row. `group_enabled` gates the whole group: a disabled group grants nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupPermissions {
    /// Whether the group is enabled at all. A disabled group denies everything.
    pub group_enabled: bool,
    pub can_create_tickets: bool,
    pub can_post_reply: bool,
    /// @implements BS-021.1: close/reopen permission.
    pub can_close_tickets: bool,
    /// @implements BS-021.1: assign/claim permission.
    pub can_assign_tickets: bool,
    /// @implements BS-021.1: transfer permission.
    pub can_transfer_tickets: bool,
    /// @implements BS-021.1: delete permission.
    pub can_delete_tickets: bool,
    /// @implements BS-021.1: edit permission.
    pub can_edit_tickets: bool,
    /// @implements BS-021.1: mass manage permission (FS-021.21).
    pub can_manage_tickets: bool,
    /// @implements FS-032.1: FAQ / knowledgebase management (M4 capability).
    pub can_manage_faq: bool,
    /// @implements FS-032.1: canned/premade-response management (M4 capability).
    pub can_manage_premade: bool,
    /// @implements FS-032.1: ban-list management (M4 capability).
    pub can_ban_emails: bool,
    /// @implements FS-032.1: view staff statistics (M4 capability).
    pub can_view_staff_stats: bool,
}

/// Why a permission check failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PermissionDenied {
    /// The staff member's group is disabled (grants nothing).
    #[error("permission group is disabled")]
    GroupDisabled,
    /// The named permission is not held by the group.
    #[error("missing permission: {0}")]
    Missing(String),
    /// The permission name is not one this build knows about.
    #[error("unknown permission: {0}")]
    Unknown(String),
}

impl GroupPermissions {
    /// Whether the group grants the **named** permission.
    ///
    /// Returns `Ok(())` when granted, or a typed [`PermissionDenied`] otherwise.
    /// A disabled group always denies. An unrecognised name denies as
    /// [`PermissionDenied::Unknown`] (fail closed).
    ///
    /// @implements BS-001: generic named-permission gate (not hardcoded).
    pub fn require(&self, permission: &str) -> Result<(), PermissionDenied> {
        if !self.group_enabled {
            return Err(PermissionDenied::GroupDisabled);
        }
        let granted = match permission {
            PERM_CAN_CREATE_TICKETS => self.can_create_tickets,
            PERM_CAN_POST_REPLY => self.can_post_reply,
            PERM_CAN_CLOSE_TICKETS => self.can_close_tickets,
            PERM_CAN_ASSIGN_TICKETS => self.can_assign_tickets,
            PERM_CAN_TRANSFER_TICKETS => self.can_transfer_tickets,
            PERM_CAN_DELETE_TICKETS => self.can_delete_tickets,
            PERM_CAN_EDIT_TICKETS => self.can_edit_tickets,
            PERM_CAN_MANAGE_TICKETS => self.can_manage_tickets,
            PERM_CAN_MANAGE_FAQ => self.can_manage_faq,
            PERM_CAN_MANAGE_PREMADE => self.can_manage_premade,
            PERM_CAN_BAN_EMAILS => self.can_ban_emails,
            PERM_CAN_VIEW_STAFF_STATS => self.can_view_staff_stats,
            other => return Err(PermissionDenied::Unknown(other.to_string())),
        };
        if granted {
            Ok(())
        } else {
            Err(PermissionDenied::Missing(permission.to_string()))
        }
    }

    /// Boolean convenience wrapper over [`require`].
    pub fn has(&self, permission: &str) -> bool {
        self.require(permission).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perms(enabled: bool, create: bool, reply: bool) -> GroupPermissions {
        GroupPermissions {
            group_enabled: enabled,
            can_create_tickets: create,
            can_post_reply: reply,
            can_close_tickets: false,
            can_assign_tickets: false,
            can_transfer_tickets: false,
            can_delete_tickets: false,
            can_edit_tickets: false,
            can_manage_tickets: false,
            can_manage_faq: false,
            can_manage_premade: false,
            can_ban_emails: false,
            can_view_staff_stats: false,
        }
    }

    /// AC-4 anchor: the generic gate allows/denies purely on the named
    /// permission. Named so `cargo test -p ost_core permission_gate` selects it.
    #[test]
    fn permission_gate_allows_and_denies_on_named_permission() {
        let granting = perms(true, false, true);
        let denying = perms(true, false, false);
        assert!(granting.has(PERM_CAN_POST_REPLY), "grants when group holds it");
        assert!(!denying.has(PERM_CAN_POST_REPLY), "denies when group lacks it");
    }

    #[test]
    fn grants_when_named_permission_held() {
        let p = perms(true, false, true);
        assert!(p.require(PERM_CAN_POST_REPLY).is_ok());
        assert!(p.has(PERM_CAN_POST_REPLY));
    }

    #[test]
    fn denies_when_named_permission_absent() {
        let p = perms(true, true, false);
        assert_eq!(
            p.require(PERM_CAN_POST_REPLY),
            Err(PermissionDenied::Missing(PERM_CAN_POST_REPLY.to_string()))
        );
        assert!(!p.has(PERM_CAN_POST_REPLY));
        // ...but a different named permission this group DOES hold is granted,
        // proving the gate keys on the name, not a hardcoded check.
        assert!(p.has(PERM_CAN_CREATE_TICKETS));
    }

    #[test]
    fn disabled_group_grants_nothing() {
        let p = perms(false, true, true);
        assert_eq!(p.require(PERM_CAN_POST_REPLY), Err(PermissionDenied::GroupDisabled));
        assert!(!p.has(PERM_CAN_CREATE_TICKETS));
    }

    /// @implements FS-032.1: the four M4 capability names resolve through the
    /// generic gate (granted iff the group holds them; fail-closed otherwise).
    #[test]
    fn m4_capability_permissions_resolve() {
        let mut p = perms(true, false, false);
        p.can_manage_faq = true;
        assert!(p.has(PERM_CAN_MANAGE_FAQ), "granted when group holds it");
        assert!(!p.has(PERM_CAN_MANAGE_PREMADE), "denied when group lacks it");
        assert!(!p.has(PERM_CAN_BAN_EMAILS));
        assert!(!p.has(PERM_CAN_VIEW_STAFF_STATS));
    }

    #[test]
    fn unknown_permission_fails_closed() {
        let p = perms(true, true, true);
        assert_eq!(
            p.require("can_delete_universe"),
            Err(PermissionDenied::Unknown("can_delete_universe".to_string()))
        );
    }
}
