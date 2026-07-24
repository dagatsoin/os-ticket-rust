//! CSRF double-submit enforcement for authenticated mutating routes (TS-M1-A4b).
//!
//! @implements BS-001: reject a state-changing authenticated request without a
//!   valid CSRF token; accept one with a matching double-submit token (ROADMAP
//!   Decision 2).
//!
//! These extractors compose the realm gate (authentication) with the CSRF check.
//! CSRF is verified **only on mutating methods** (POST/PUT/PATCH/DELETE); safe
//! methods (GET/HEAD/OPTIONS) authenticate without a CSRF token. Unauthenticated
//! public endpoints never use these extractors, so they are CSRF-exempt by
//! construction (the two login POSTs + `POST /api/tickets`).

use async_trait::async_trait;
use axum::extract::FromRequestParts;
use http::request::Parts;
use http::Method;

use ost_core::{verify_double_submit, ApiError, Realm, CSRF_HEADER};

use crate::auth::cookies::{csrf_cookie_name, read_cookie};
use crate::auth::realm::{ClientSession, StaffSession};

/// Whether a method mutates state (and therefore requires a CSRF token).
fn is_mutating(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

/// Verify the double-submit pair for a realm against the request parts.
/// Returns a 403 `ApiError` on any failure.
fn enforce_csrf(parts: &Parts, realm: Realm) -> Result<(), ApiError> {
    if !is_mutating(&parts.method) {
        return Ok(());
    }
    let cookie = read_cookie(&parts.headers, csrf_cookie_name(realm));
    let header = parts
        .headers
        .get(CSRF_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    if verify_double_submit(cookie.as_deref(), header.as_deref()) {
        Ok(())
    } else {
        Err(ApiError::forbidden("CSRF token missing or invalid"))
    }
}

/// Staff realm session **with CSRF enforced** on mutating methods. Use this on
/// authenticated staff mutating routes (e.g. staff reply); use the bare
/// [`StaffSession`] on read-only staff routes.
#[derive(Debug, Clone)]
pub struct StaffCsrf(pub StaffSession);

/// Client realm session **with CSRF enforced** on mutating methods.
#[derive(Debug, Clone)]
pub struct ClientCsrf(pub ClientSession);

#[async_trait]
impl FromRequestParts<crate::state::AppState> for StaffCsrf {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::state::AppState,
    ) -> Result<Self, Self::Rejection> {
        // Authenticate first (401 takes precedence over CSRF 403).
        let session = StaffSession::from_request_parts(parts, state).await?;
        enforce_csrf(parts, Realm::Staff)?;
        Ok(StaffCsrf(session))
    }
}

#[async_trait]
impl FromRequestParts<crate::state::AppState> for ClientCsrf {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::state::AppState,
    ) -> Result<Self, Self::Rejection> {
        let session = ClientSession::from_request_parts(parts, state).await?;
        enforce_csrf(parts, Realm::Client)?;
        Ok(ClientCsrf(session))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{HeaderMap, HeaderValue};

    fn parts_with(method: Method, headers: HeaderMap) -> Parts {
        let mut req = http::Request::builder().method(method).uri("/x");
        *req.headers_mut().unwrap() = headers;
        let (parts, ()) = req.body(()).unwrap().into_parts();
        parts
    }

    #[test]
    fn get_is_exempt_from_csrf() {
        let parts = parts_with(Method::GET, HeaderMap::new());
        assert!(enforce_csrf(&parts, Realm::Staff).is_ok());
    }

    #[test]
    fn post_without_token_is_rejected() {
        let parts = parts_with(Method::POST, HeaderMap::new());
        let err = enforce_csrf(&parts, Realm::Staff).unwrap_err();
        assert_eq!(err.status, http::StatusCode::FORBIDDEN);
    }

    #[test]
    fn post_with_matching_token_is_accepted() {
        let mut h = HeaderMap::new();
        h.insert(
            http::header::COOKIE,
            HeaderValue::from_static("XSRF-TOKEN-STAFF=tok123"),
        );
        h.insert(CSRF_HEADER, HeaderValue::from_static("tok123"));
        let parts = parts_with(Method::POST, h);
        assert!(enforce_csrf(&parts, Realm::Staff).is_ok());
    }

    #[test]
    fn post_with_mismatched_token_is_rejected() {
        let mut h = HeaderMap::new();
        h.insert(
            http::header::COOKIE,
            HeaderValue::from_static("XSRF-TOKEN-STAFF=tok123"),
        );
        h.insert(CSRF_HEADER, HeaderValue::from_static("WRONG"));
        let parts = parts_with(Method::POST, h);
        assert!(enforce_csrf(&parts, Realm::Staff).is_err());
    }
}
