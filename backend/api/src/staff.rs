//! Staff-realm read/profile routes (TS-M1-C1/C2/C3).
//!
//! All routes here are gated by the staff realm (the [`StaffSession`] extractor,
//! TS-M1-A4b); mutating routes additionally enforce CSRF + a named permission.
//!
//! @implements BS-002: authenticated staff profile (`GET /api/staff/me`).

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use ost_core::ApiError;

use crate::auth::realm::StaffSession;
use crate::state::AppState;

/// The authenticated staff profile returned by `GET /api/staff/me`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffMe {
    pub id: i32,
    pub username: String,
    /// Display name (`firstname lastname`, trimmed; falls back to username).
    pub name: String,
    pub dept_id: i32,
}

/// `GET /api/staff/me` — return the authenticated staff account profile.
///
/// Read-only (no CSRF). The staff realm gate yields a 401 when no valid staff
/// session is present.
///
/// @implements BS-002: staff profile lookup `{ id, username, name, deptId }`.
pub async fn me(
    State(state): State<AppState>,
    session: StaffSession,
) -> Result<Json<StaffMe>, ApiError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("Profile lookup is unavailable"))?;

    let row: Option<(String, String, String, i32)> = sqlx::query_as(
        "SELECT username, firstname, lastname, dept_id FROM staff WHERE staff_id = $1",
    )
    .bind(session.staff_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal("Profile lookup failed"))?;

    // A valid session whose account no longer exists is treated as invalid.
    let (username, firstname, lastname, dept_id) =
        row.ok_or_else(|| ApiError::unauthenticated("Session expired or invalid"))?;

    let name = display_name(&firstname, &lastname, &username);

    Ok(Json(StaffMe {
        id: session.staff_id,
        username,
        name,
        dept_id,
    }))
}

/// Compose a display name from first/last, falling back to the username when
/// both name parts are blank.
fn display_name(firstname: &str, lastname: &str, username: &str) -> String {
    let full = format!("{} {}", firstname.trim(), lastname.trim());
    let full = full.trim().to_string();
    if full.is_empty() {
        username.to_string()
    } else {
        full
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_composes_or_falls_back() {
        assert_eq!(display_name("Agent", "One", "agent"), "Agent One");
        assert_eq!(display_name("", "", "agent"), "agent");
        assert_eq!(display_name("  ", "Solo", "agent"), "Solo");
    }
}
