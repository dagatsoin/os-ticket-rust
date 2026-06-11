//! `core` — shared domain primitives for the osTicket modernisation backend.
//!
//! Currently exposes the shared JSON error envelope ([`error`]). Future M1
//! tickets add domain types here (the ticket create-and-append core, validation
//! helpers, etc.) so HTTP adapters stay thin.

pub mod error;

pub use error::{ApiError, ErrorBody, ErrorEnvelope};
