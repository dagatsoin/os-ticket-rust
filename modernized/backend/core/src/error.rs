//! Shared JSON error envelope for the osTicket modernisation backend.
//!
//! @implements ROADMAP M1 Decision 4 — shared JSON error envelope contract.
//! Owned by TS-M1-A1; every route emits this shape and the frontend apiClient
//! (TS-M1-A5) parses it.
//!
//! Envelope shape:
//! ```json
//! { "error": { "message": "<top-level>", "fields": { "<field>": "<msg>" } } }
//! ```
//!
//! Status codes: 422 validation failures, 401 unauthenticated, 403 forbidden,
//! 404 not found (500 for unexpected internal failures).

use std::collections::BTreeMap;

use axum::response::{IntoResponse, Response};
use axum::Json;
use http::StatusCode;
use serde::Serialize;

/// A single application error carrying an HTTP status, a human-readable
/// top-level message, and an optional per-field validation map.
///
/// This is the one error type all handlers should return. It serialises into
/// the shared envelope via [`IntoResponse`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct ApiError {
    /// HTTP status code emitted for this error.
    pub status: StatusCode,
    /// Top-level, human-readable error message.
    pub message: String,
    /// Optional per-field messages (used primarily for 422 validation errors).
    pub fields: BTreeMap<String, String>,
}

impl ApiError {
    /// Construct an error with an explicit status and message and no field map.
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            fields: BTreeMap::new(),
        }
    }

    /// 422 Unprocessable Entity — validation failure with no field detail yet.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, message)
    }

    /// 401 Unauthorized — request is not authenticated.
    pub fn unauthenticated(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }

    /// 403 Forbidden — authenticated but not permitted.
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, message)
    }

    /// 400 Bad Request — malformed or invalid input.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    /// 404 Not Found.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    /// 500 Internal Server Error — unexpected failure.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    /// 409 Conflict — resource state conflict (e.g., ticket locked by another).
    ///
    /// @implements FS-021.18: Lock conflict response.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, message)
    }

    /// Attach a single field-level message (chainable). Typically used to build
    /// up a 422 validation response.
    #[must_use]
    pub fn with_field(mut self, field: impl Into<String>, message: impl Into<String>) -> Self {
        self.fields.insert(field.into(), message.into());
        self
    }

    /// Replace the whole field map (chainable).
    #[must_use]
    pub fn with_fields(mut self, fields: BTreeMap<String, String>) -> Self {
        self.fields = fields;
        self
    }

    /// Render the error body as the shared envelope value (without the HTTP
    /// status). Exposed for tests and for callers that need the raw JSON.
    pub fn to_envelope(&self) -> ErrorEnvelope {
        ErrorEnvelope {
            error: ErrorBody {
                message: self.message.clone(),
                fields: self.fields.clone(),
            },
        }
    }
}

/// Top-level serialised wrapper: `{ "error": { ... } }`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

/// Inner serialised body: `{ "message": "...", "fields": { ... } }`.
///
/// `fields` is always present (an empty object when there are no field errors)
/// so the frontend can rely on a stable shape.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ErrorBody {
    pub message: String,
    pub fields: BTreeMap<String, String>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status;
        let body = self.to_envelope();
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_has_error_message_and_empty_fields_object() {
        let err = ApiError::not_found("missing");
        let value = serde_json::to_value(err.to_envelope()).unwrap();

        assert_eq!(value["error"]["message"], "missing");
        // fields must serialise as an object, even when empty.
        assert!(value["error"]["fields"].is_object());
        assert_eq!(value["error"]["fields"].as_object().unwrap().len(), 0);
    }

    #[test]
    fn validation_error_serialises_field_map() {
        let err = ApiError::validation("Validation failed")
            .with_field("email", "must be a valid email")
            .with_field("name", "is required");
        let value = serde_json::to_value(err.to_envelope()).unwrap();

        assert_eq!(value["error"]["message"], "Validation failed");
        assert_eq!(value["error"]["fields"]["email"], "must be a valid email");
        assert_eq!(value["error"]["fields"]["name"], "is required");
    }

    #[test]
    fn status_helpers_map_to_expected_codes() {
        assert_eq!(
            ApiError::validation("x").status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            ApiError::unauthenticated("x").status,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(ApiError::forbidden("x").status, StatusCode::FORBIDDEN);
        assert_eq!(ApiError::not_found("x").status, StatusCode::NOT_FOUND);
        assert_eq!(
            ApiError::internal("x").status,
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(ApiError::conflict("x").status, StatusCode::CONFLICT);
    }
}
