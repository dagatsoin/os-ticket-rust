//! Login / logout routes for both realms (TS-M1-A4b).
//!
//! @implements BS-001: staff login establishes an account-scoped session +
//!   seeds the staff CSRF cookie; logout destroys the session.
//! @implements BS-002: client login establishes a ticket-scoped session
//!   (ticket# + email, BS-091.1 identity) + seeds the client CSRF cookie.
//! @implements FS-002.14: session-id regeneration on login (a fresh id is minted
//!   each login; any prior cookie is superseded — fixation defence).
//!
//! On success the response sets TWO cookies: the HttpOnly `ost_*_sess` session
//! cookie and the non-HttpOnly `XSRF-TOKEN-*` CSRF cookie (double-submit seed).

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use http::{HeaderMap, StatusCode};
use serde::Deserialize;

use ost_core::session::{SessionData, SESSION_TTL_SECS};
use ost_core::{new_csrf_token, verify_password, ApiError, Realm};

use crate::auth::cookies::{append_set_cookie, build_csrf_cookie, build_session_cookie, clear_cookie, csrf_cookie_name, session_cookie_name};
use crate::auth::csrf::{ClientCsrf, StaffCsrf};
use crate::state::AppState;

/// Staff login request body.
#[derive(Debug, Deserialize)]
pub struct StaffLoginRequest {
    pub username: String,
    pub password: String,
}

/// Client login request body (ticket-scoped identity).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientLoginRequest {
    pub ticket_number: String,
    pub email: String,
}

/// Build the success response for a freshly established session: 200 + both
/// cookies (session + CSRF). The CSRF token is also returned in the body so a
/// non-cookie client can read it for the `X-CSRFToken` header.
fn login_success(realm: Realm, session_id: &str, secure: bool) -> Response {
    let csrf = new_csrf_token();
    let mut headers = HeaderMap::new();
    append_set_cookie(
        &mut headers,
        build_session_cookie(realm, session_id, SESSION_TTL_SECS, secure),
    );
    append_set_cookie(
        &mut headers,
        build_csrf_cookie(realm, &csrf, SESSION_TTL_SECS, secure),
    );
    let body = Json(serde_json::json!({ "ok": true, "csrfToken": csrf }));
    (StatusCode::OK, headers, body).into_response()
}

/// `POST /api/staff/login` — verify credentials, establish a staff session.
///
/// CSRF-exempt (unauthenticated public endpoint). On success: a NEW session id
/// is minted (regeneration) and the staff session + CSRF cookies are set.
///
/// @implements BS-001 / FS-002.14: staff login + session regeneration.
pub async fn staff_login(
    State(state): State<AppState>,
    Json(req): Json<StaffLoginRequest>,
) -> Result<Response, ApiError> {
    let (pool, store) = state
        .pool
        .as_ref()
        .zip(state.sessions.as_ref())
        .ok_or_else(|| ApiError::internal("Authentication is unavailable"))?;

    // Generic credentials error (don't leak whether the username exists).
    let bad = || ApiError::unauthenticated("Invalid username or password");

    let row: Option<(i32, String, bool)> =
        sqlx::query_as("SELECT staff_id, passwd, isactive FROM staff WHERE username = $1")
            .bind(&req.username)
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::internal("Login failed"))?;

    let (staff_id, passwd, isactive) = row.ok_or_else(bad)?;
    if !isactive {
        return Err(bad());
    }
    let ok = verify_password(&req.password, &passwd).map_err(|_| bad())?;
    if !ok {
        return Err(bad());
    }

    // Regeneration: minting a new session row yields a brand-new id.
    let session = store
        .create(&SessionData::Staff { staff_id })
        .await
        .map_err(|_| ApiError::internal("Could not establish a session"))?;

    Ok(login_success(Realm::Staff, &session.id, state.app_env.cookies_secure()))
}

/// `POST /api/client/login` — verify ticket# + email, establish a client session
/// scoped to that one ticket.
///
/// CSRF-exempt (unauthenticated public endpoint). Implements the FS-010.3
/// validation order:
///
/// 1. The email must be syntactically valid and the ticket number non-empty,
///    otherwise the attempt fails **without a database lookup**.
/// 2. The ticket is looked up by external number + email together; it must
///    exist (the email match is **case-insensitive**, FS-010.3 step 3).
///
/// A single generic error is surfaced on any failure (no oracle on which of
/// ticket-number / email was wrong).
///
/// @implements FS-010.3: interactive client login (validation order, case-
///   insensitive email match).
/// @implements BS-002 / FS-002.14: client session + session-id regeneration.
pub async fn client_login(
    State(state): State<AppState>,
    Json(req): Json<ClientLoginRequest>,
) -> Result<Response, ApiError> {
    let (pool, store) = state
        .pool
        .as_ref()
        .zip(state.sessions.as_ref())
        .ok_or_else(|| ApiError::internal("Authentication is unavailable"))?;

    // Single generic error for every failure path (FS-010.3 — no field oracle).
    let bad = || ApiError::unauthenticated("Authentication error - try again!");

    // FS-010.3 step 1: email must be syntactically valid AND ticket# non-empty,
    // BEFORE any DB lookup.
    let email = req.email.trim().to_string();
    let raw_number = req.ticket_number.trim();
    if raw_number.is_empty() || !ost_core::is_email(&email) {
        return Err(bad());
    }
    // The external ticket number is numeric (FS-091.2); a non-numeric value
    // cannot match any ticket — still the generic error, no DB lookup needed.
    let ticket_number: i64 = raw_number.parse().map_err(|_| bad())?;

    // FS-010.3 step 2+3: look up by (external number, email) together; the email
    // match is case-insensitive. Return the canonical stored email for the
    // session payload (so the bound identity matches the persisted casing).
    let matched: Option<(String,)> = sqlx::query_as(
        r#"SELECT email FROM ticket WHERE "ticketID" = $1 AND lower(email) = lower($2)"#,
    )
    .bind(ticket_number)
    .bind(&email)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal("Login failed"))?;
    let stored_email = matched.ok_or_else(bad)?.0;

    let session = store
        .create(&SessionData::Client {
            ticket_number,
            email: stored_email,
        })
        .await
        .map_err(|_| ApiError::internal("Could not establish a session"))?;

    Ok(login_success(Realm::Client, &session.id, state.app_env.cookies_secure()))
}

/// `POST /api/staff/logout` — destroy the staff session + clear both cookies.
///
/// A mutating authenticated route: CSRF-protected via the [`StaffCsrf`]
/// extractor (so it also serves as the AC-1 reference flow).
pub async fn staff_logout(
    State(state): State<AppState>,
    StaffCsrf(session): StaffCsrf,
) -> Result<Response, ApiError> {
    if let Some(store) = state.sessions.as_ref() {
        let _ = store.destroy(&session.session_id).await;
    }
    Ok(logout_success(Realm::Staff))
}

/// `POST /api/client/logout` — destroy the client session + clear both cookies.
pub async fn client_logout(
    State(state): State<AppState>,
    ClientCsrf(session): ClientCsrf,
) -> Result<Response, ApiError> {
    if let Some(store) = state.sessions.as_ref() {
        let _ = store.destroy(&session.session_id).await;
    }
    Ok(logout_success(Realm::Client))
}

/// Clear both the session and CSRF cookies for a realm and return 200.
fn logout_success(realm: Realm) -> Response {
    let mut headers = HeaderMap::new();
    append_set_cookie(&mut headers, clear_cookie(session_cookie_name(realm)));
    append_set_cookie(&mut headers, clear_cookie(csrf_cookie_name(realm)));
    (StatusCode::OK, headers, Json(serde_json::json!({ "ok": true }))).into_response()
}
