//! Cookie helpers for the two realms (TS-M1-A4b).
//!
//! @implements ROADMAP Decision 1 — two distinct session cookies, one per realm
//!   (`ost_staff_sess` / `ost_client_sess`), both `HttpOnly`, `SameSite=Lax`,
//!   `Secure` in non-dev.
//! @implements ROADMAP Decision 2 — non-HttpOnly `XSRF-TOKEN-{STAFF,CLIENT}`
//!   cookies seeded at login (readable by the SPA so it can reflect the value
//!   into the `X-CSRFToken` header).
//!
//! Cookie building/parsing is hand-rolled (no extra crate) so the exact security
//! attributes are explicit and under test.

use http::header::{COOKIE, SET_COOKIE};
use http::{HeaderMap, HeaderValue};

use ost_core::Realm;

/// Staff realm session cookie name (HttpOnly).
pub const STAFF_SESSION_COOKIE: &str = "ost_staff_sess";
/// Client realm session cookie name (HttpOnly).
pub const CLIENT_SESSION_COOKIE: &str = "ost_client_sess";

/// The session cookie name for a realm.
pub fn session_cookie_name(realm: Realm) -> &'static str {
    match realm {
        Realm::Staff => STAFF_SESSION_COOKIE,
        Realm::Client => CLIENT_SESSION_COOKIE,
    }
}

/// The CSRF cookie name for a realm.
pub fn csrf_cookie_name(realm: Realm) -> &'static str {
    match realm {
        Realm::Staff => ost_core::STAFF_CSRF_COOKIE,
        Realm::Client => ost_core::CLIENT_CSRF_COOKIE,
    }
}

/// Read a cookie value by name from the request headers. Returns the first match
/// (cookies are a single `Cookie:` header of `name=value; name2=value2` pairs).
pub fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(COOKIE)?.to_str().ok()?;
    for pair in raw.split(';') {
        let pair = pair.trim();
        if let Some((k, v)) = pair.split_once('=') {
            if k.trim() == name {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

/// Build a `Set-Cookie` value for the HttpOnly **session** cookie.
///
/// Attributes: `HttpOnly`, `SameSite=Lax`, `Path=/`, `Max-Age=<ttl>`, and
/// `Secure` only when `secure` is set (production). The SPA cannot read this
/// cookie (HttpOnly), which is why a separate non-HttpOnly CSRF cookie exists.
pub fn build_session_cookie(realm: Realm, value: &str, ttl_secs: i64, secure: bool) -> HeaderValue {
    let mut s = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        session_cookie_name(realm),
        value,
        ttl_secs
    );
    if secure {
        s.push_str("; Secure");
    }
    HeaderValue::from_str(&s).expect("session cookie is header-safe")
}

/// Build a `Set-Cookie` value for the non-HttpOnly **CSRF** cookie.
///
/// Deliberately **not** `HttpOnly` (the SPA must read it to reflect into the
/// `X-CSRFToken` header). Still `SameSite=Lax` + `Secure` in production.
pub fn build_csrf_cookie(realm: Realm, value: &str, ttl_secs: i64, secure: bool) -> HeaderValue {
    let mut s = format!(
        "{}={}; Path=/; SameSite=Lax; Max-Age={}",
        csrf_cookie_name(realm),
        value,
        ttl_secs
    );
    if secure {
        s.push_str("; Secure");
    }
    HeaderValue::from_str(&s).expect("csrf cookie is header-safe")
}

/// Build a `Set-Cookie` value that **clears** a cookie (logout): empty value +
/// `Max-Age=0`.
pub fn clear_cookie(name: &str) -> HeaderValue {
    let s = format!("{name}=; Path=/; Max-Age=0");
    HeaderValue::from_str(&s).expect("clear cookie is header-safe")
}

/// Append a `Set-Cookie` header to a header map (multiple cookies append, not
/// overwrite).
pub fn append_set_cookie(headers: &mut HeaderMap, value: HeaderValue) {
    headers.append(SET_COOKIE, value);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers_with_cookie(raw: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(COOKIE, HeaderValue::from_str(raw).unwrap());
        h
    }

    #[test]
    fn reads_named_cookie_among_many() {
        let h = headers_with_cookie("a=1; ost_staff_sess=abc123; b=2");
        assert_eq!(read_cookie(&h, "ost_staff_sess").as_deref(), Some("abc123"));
        assert_eq!(read_cookie(&h, "a").as_deref(), Some("1"));
        assert_eq!(read_cookie(&h, "missing"), None);
    }

    #[test]
    fn read_cookie_absent_header_is_none() {
        let h = HeaderMap::new();
        assert_eq!(read_cookie(&h, "x"), None);
    }

    #[test]
    fn session_cookie_is_httponly_lax_and_secure_only_in_prod() {
        let dev = build_session_cookie(Realm::Staff, "v", 86400, false);
        let dev = dev.to_str().unwrap();
        assert!(dev.starts_with("ost_staff_sess=v"));
        assert!(dev.contains("HttpOnly"));
        assert!(dev.contains("SameSite=Lax"));
        assert!(dev.contains("Max-Age=86400"));
        assert!(!dev.contains("Secure"), "dev cookie must not be Secure");

        let prod = build_session_cookie(Realm::Client, "v", 86400, true);
        assert!(prod.to_str().unwrap().contains("Secure"));
        assert!(prod.to_str().unwrap().starts_with("ost_client_sess=v"));
    }

    #[test]
    fn csrf_cookie_is_not_httponly() {
        let c = build_csrf_cookie(Realm::Staff, "tok", 86400, false);
        let c = c.to_str().unwrap();
        assert!(c.starts_with("XSRF-TOKEN-STAFF=tok"));
        assert!(!c.contains("HttpOnly"), "CSRF cookie must be readable by JS");
        assert!(c.contains("SameSite=Lax"));
    }

    #[test]
    fn clear_cookie_expires_immediately() {
        let c = clear_cookie("ost_staff_sess");
        assert!(c.to_str().unwrap().contains("Max-Age=0"));
    }
}
