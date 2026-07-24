//! Realm-gate extractors for `/api/staff/**` and `/api/client/**` (TS-M1-A4b).
//!
//! @implements BS-001: staff realm gate (account-scoped session).
//! @implements BS-002: client realm gate (ticket-scoped session).
//!
//! Each extractor reads ONLY its realm's cookie, loads + validates the session
//! against the DB store, and confirms the stored payload is for the matching
//! realm. Anything missing / expired / wrong-realm yields a 401 error envelope.
//! The cookies are never shared across realms (ROADMAP Decision 1), so a client
//! cookie presented to a staff route simply isn't read, and vice versa.

use async_trait::async_trait;
use axum::extract::FromRequestParts;
use http::request::Parts;

use ost_core::session::SessionData;
use ost_core::{ApiError, Realm, SortPreferences};

use crate::auth::cookies::{read_cookie, session_cookie_name};
use crate::state::AppState;

/// An authenticated **staff** session (account-scoped). Extracting it gates a
/// route to the staff realm.
#[derive(Debug, Clone)]
pub struct StaffSession {
    /// The session id (cookie value) — needed for CSRF + logout.
    pub session_id: String,
    /// The authenticated staff account id.
    pub staff_id: i32,
    /// Sticky sort preferences per queue (BS-020.8).
    pub sort_prefs: Option<SortPreferences>,
}

/// An authenticated **client** session (ticket-scoped). Extracting it gates a
/// route to the client realm.
#[derive(Debug, Clone)]
pub struct ClientSession {
    /// The session id (cookie value).
    pub session_id: String,
    /// External ticket number this session is scoped to.
    pub ticket_number: i64,
    /// Requester email bound to the ticket.
    pub email: String,
}

/// Shared loading: read the realm cookie, load + validate the session, and
/// return the typed payload (or a 401). Centralises the "absent/expired ⇒ 401"
/// behaviour for both realms.
async fn load_realm_session(
    parts: &Parts,
    state: &AppState,
    realm: Realm,
) -> Result<(String, SessionData), ApiError> {
    let store = state
        .sessions
        .as_ref()
        .ok_or_else(|| ApiError::unauthenticated("Authentication required"))?;

    let cookie_name = session_cookie_name(realm);
    let session_id = read_cookie(&parts.headers, cookie_name)
        .ok_or_else(|| ApiError::unauthenticated("Authentication required"))?;

    let session = store
        .load(&session_id)
        .await
        .map_err(|_| ApiError::internal("Session lookup failed"))?
        .ok_or_else(|| ApiError::unauthenticated("Session expired or invalid"))?;

    // The cookie is realm-specific, but defend in depth: the stored payload must
    // also be for this realm.
    if session.data.realm() != realm {
        return Err(ApiError::unauthenticated("Session expired or invalid"));
    }
    Ok((session_id, session.data))
}

#[async_trait]
impl FromRequestParts<AppState> for StaffSession {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let (session_id, data) = load_realm_session(parts, state, Realm::Staff).await?;
        match data {
            SessionData::Staff { staff_id, sort_prefs } => Ok(StaffSession {
                session_id,
                staff_id,
                sort_prefs,
            }),
            // Realm already validated above; unreachable in practice.
            _ => Err(ApiError::unauthenticated("Session expired or invalid")),
        }
    }
}

#[async_trait]
impl FromRequestParts<AppState> for ClientSession {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let (session_id, data) = load_realm_session(parts, state, Realm::Client).await?;
        match data {
            SessionData::Client {
                ticket_number,
                email,
            } => Ok(ClientSession {
                session_id,
                ticket_number,
                email,
            }),
            _ => Err(ApiError::unauthenticated("Session expired or invalid")),
        }
    }
}
