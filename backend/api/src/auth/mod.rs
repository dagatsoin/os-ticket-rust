//! Authentication / authorization HTTP layer (TS-M1-A4b).
//!
//! Wires the `ost_core` stateful tier (DB-backed sessions, CSRF double-submit,
//! the named-permission gate) into Axum: realm cookies, realm-gate extractors,
//! CSRF-enforcing extractors, the permission-gate helper, and the login/logout
//! routes. See ROADMAP Decisions 1 & 2 for the pinned auth/CSRF model.

pub mod cookies;
pub mod csrf;
pub mod gate;
pub mod realm;
pub mod routes;

pub use csrf::{ClientCsrf, StaffCsrf};
pub use gate::require_staff_permission;
pub use realm::{ClientSession, StaffSession};
