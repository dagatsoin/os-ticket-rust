//! `core` — shared domain primitives for the osTicket modernisation backend.
//!
//! Exposes the shared JSON error envelope ([`error`]) plus the TS-M1-A4a
//! pure-function tier: input [`validation`], HTML [`sanitize`]-ation, and
//! password [`hashing`] (argon2id). HTTP adapters stay thin by reusing these.

pub mod error;
pub mod hashing;
pub mod sanitize;
pub mod validation;

pub use error::{ApiError, ErrorBody, ErrorEnvelope};
pub use hashing::{hash_password, verify_password, HashError};
pub use sanitize::{safe_html, sanitize};
pub use validation::{
    is_email, validate_email_field, validate_password, validate_required, FieldError,
    PASSWORD_MIN_LEN,
};
