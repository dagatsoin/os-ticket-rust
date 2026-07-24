//! CSRF double-submit token verification (TS-M1-A4b).
//!
//! @implements BS-001: CSRF protection on authenticated mutating requests.
//! @implements FS-001.11 / FS-002.7 (modernised): the legacy server-rendered
//!   hidden-field CSRF token is replaced by the SPA **double-submit** model
//!   (ROADMAP Decision 2). On login the server seeds a non-HttpOnly
//!   `XSRF-TOKEN-{STAFF,CLIENT}` cookie; the apiClient reflects it into an
//!   `X-CSRFToken` header on every mutating request; this module verifies the
//!   header value equals the cookie value (constant-time).
//!
//! No per-request rotation in M1 (re-seed at login only). Unauthenticated public
//! endpoints are exempt (the gate is only applied on authenticated routes).

use password_hash::rand_core::{OsRng, RngCore};

/// The non-HttpOnly cookie name carrying the CSRF token for the staff realm.
pub const STAFF_CSRF_COOKIE: &str = "XSRF-TOKEN-STAFF";
/// The non-HttpOnly cookie name carrying the CSRF token for the client realm.
pub const CLIENT_CSRF_COOKIE: &str = "XSRF-TOKEN-CLIENT";
/// The request header the frontend reflects the cookie value into.
pub const CSRF_HEADER: &str = "X-CSRFToken";

/// Mint a fresh CSRF token (128 bits of entropy, hex-encoded). Seeded into the
/// `XSRF-TOKEN-*` cookie at login.
pub fn new_csrf_token() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    let mut s = String::with_capacity(32);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Constant-time equality of two byte slices (length-leaking only).
///
/// Avoids a short-circuit timing oracle when comparing the header token against
/// the cookie token.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Verify a double-submit CSRF pair: the `X-CSRFToken` header value must be
/// present, non-empty, and equal to the realm's `XSRF-TOKEN-*` cookie value.
///
/// Returns `true` only when both are present and match. A missing header, a
/// missing cookie, or a mismatch all return `false` (caller emits 403).
///
/// @implements BS-001: double-submit verification.
pub fn verify_double_submit(cookie_token: Option<&str>, header_token: Option<&str>) -> bool {
    match (cookie_token, header_token) {
        (Some(c), Some(h)) if !c.is_empty() && !h.is_empty() => {
            constant_time_eq(c.as_bytes(), h.as_bytes())
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_matching_non_empty_pair() {
        assert!(verify_double_submit(Some("abc123"), Some("abc123")));
    }

    #[test]
    fn rejects_mismatch() {
        assert!(!verify_double_submit(Some("abc123"), Some("abc124")));
    }

    #[test]
    fn rejects_missing_header_or_cookie() {
        assert!(!verify_double_submit(Some("abc123"), None));
        assert!(!verify_double_submit(None, Some("abc123")));
        assert!(!verify_double_submit(None, None));
    }

    #[test]
    fn rejects_empty_tokens() {
        assert!(!verify_double_submit(Some(""), Some("")));
        assert!(!verify_double_submit(Some("x"), Some("")));
        assert!(!verify_double_submit(Some(""), Some("x")));
    }

    #[test]
    fn tokens_are_unique_and_hex() {
        let a = new_csrf_token();
        let b = new_csrf_token();
        assert_ne!(a, b);
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
