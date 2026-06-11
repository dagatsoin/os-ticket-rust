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

/// The permission flags a staff member's group grants, loaded from the `groups`
/// row. `group_enabled` gates the whole group: a disabled group grants nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupPermissions {
    /// Whether the group is enabled at all. A disabled group denies everything.
    pub group_enabled: bool,
    pub can_create_tickets: bool,
    pub can_post_reply: bool,
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

    #[test]
    fn unknown_permission_fails_closed() {
        let p = perms(true, true, true);
        assert_eq!(
            p.require("can_delete_universe"),
            Err(PermissionDenied::Unknown("can_delete_universe".to_string()))
        );
    }
}
