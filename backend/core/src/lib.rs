//! `core` — shared domain primitives for the osTicket modernisation backend.
//!
//! Exposes the shared JSON error envelope ([`error`]) plus the TS-M1-A4a
//! pure-function tier: input [`validation`], HTML [`sanitize`]-ation, and
//! password [`hashing`] (argon2id). HTTP adapters stay thin by reusing these.
//!
//! TS-M1-A4b adds the stateful/middleware tier: DB-backed [`session`]s (both
//! realms), [`csrf`] double-submit verification, the generic [`permission`]
//! gate, and the stub [`mailer`] port.

pub mod csrf;
pub mod error;
pub mod hashing;
pub mod mailer;
pub mod permission;
pub mod sanitize;
pub mod session;
pub mod validation;

pub use csrf::{
    new_csrf_token, verify_double_submit, CLIENT_CSRF_COOKIE, CSRF_HEADER, STAFF_CSRF_COOKIE,
};
pub use error::{ApiError, ErrorBody, ErrorEnvelope};
pub use hashing::{hash_password, verify_password, HashError};
pub use mailer::{MailError, Mailer, OutboundMail, StubMailer};
pub use permission::{
    GroupPermissions, PermissionDenied, PERM_CAN_CREATE_TICKETS, PERM_CAN_POST_REPLY,
};
pub use sanitize::{safe_html, sanitize};
pub use session::{Realm, Session, SessionData, SessionError, SessionStore, SESSION_TTL_SECS};
pub use validation::{
    is_email, validate_email_field, validate_password, validate_required, FieldError,
    PASSWORD_MIN_LEN,
};
