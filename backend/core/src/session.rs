//! DB-backed login sessions for both realms (TS-M1-A4b).
//!
//! @implements BS-001: staff authentication / session establishment.
//! @implements BS-002: client (ticket-scoped) authentication / session.
//! @implements FS-002.14: server-side session persistence (DB-backed `session`
//!   table) — TTL 86400s, session-id regeneration on login (fixation defence).
//!
//! The legacy app stores PHP session blobs in `ost_session`. Here the same
//! `session` table backs a typed, realm-aware store. A session row carries an
//! opaque random `session_id` (the cookie value), a JSON [`SessionData`] payload
//! in `session_data`, and an absolute `session_expire` instant. Two realm shapes
//! are supported (ROADMAP Decision 1):
//!
//! * **staff** — account-scoped: identified by `staff_id`.
//! * **client** — ticket-scoped: identified by the external ticket number +
//!   the requester email (BS-091.1 composite identity).
//!
//! Both realms share the one table and one store; the realm is encoded in the
//! JSON payload and validated by the realm gates (see [`crate::realm`]).

use std::time::Duration;

use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;

/// Session time-to-live: **86400 seconds** (24h), per TS-M1-A4b / FS-002.14.
pub const SESSION_TTL_SECS: i64 = 86_400;

/// The two authentication realms. Kept deliberately separate (never shared
/// across cookies) per ROADMAP Decision 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Realm {
    /// Staff control panel — account-scoped session.
    Staff,
    /// Client portal — ticket-scoped session.
    Client,
}

impl Realm {
    /// Stable string tag persisted inside the JSON payload.
    pub fn tag(self) -> &'static str {
        match self {
            Realm::Staff => "staff",
            Realm::Client => "client",
        }
    }
}

/// The typed payload stored as JSON in `session.session_data`.
///
/// One variant per realm. The staff variant is account-scoped; the client
/// variant is ticket-scoped (external ticket number + requester email).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "realm", rename_all = "snake_case")]
pub enum SessionData {
    /// Staff (account-scoped) session.
    Staff {
        /// The authenticated staff account id.
        staff_id: i32,
    },
    /// Client (ticket-scoped) session.
    Client {
        /// External ticket number (FS-091.2 `ticketID`).
        ticket_number: i64,
        /// Requester email bound to the ticket (BS-091.1 composite identity).
        email: String,
    },
}

impl SessionData {
    /// Which realm this payload belongs to.
    pub fn realm(&self) -> Realm {
        match self {
            SessionData::Staff { .. } => Realm::Staff,
            SessionData::Client { .. } => Realm::Client,
        }
    }
}

/// A live, validated session loaded from the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// The opaque session id (also the cookie value).
    pub id: String,
    /// The typed realm payload.
    pub data: SessionData,
}

/// Errors from the session store.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("failed to (de)serialize session data: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Generate a fresh, unguessable session id (URL-safe, 256 bits of entropy).
///
/// A new id is minted on every login, so the post-login cookie value never
/// equals the pre-login one (session-fixation defence — TS-M1-A4b AC-3).
fn new_session_id() -> String {
    use password_hash::rand_core::{OsRng, RngCore};
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    // Lowercase hex — cookie-safe, no padding, fixed length.
    let mut s = String::with_capacity(64);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// A DB-backed session store over the shared `session` table.
///
/// Cloneable (wraps a [`PgPool`]); pass it into Axum state.
#[derive(Clone)]
pub struct SessionStore {
    pool: PgPool,
}

impl SessionStore {
    /// Wrap a connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new session, returning the freshly minted id.
    ///
    /// Always mints a brand-new id (callers regenerate on login by simply
    /// creating a new session and dropping the old cookie/row). The row's
    /// `session_expire` is set to `now() + SESSION_TTL_SECS`.
    ///
    /// @implements FS-002.14: session creation with TTL + fresh id (fixation).
    pub async fn create(&self, data: &SessionData) -> Result<Session, SessionError> {
        let id = new_session_id();
        let payload = serde_json::to_string(data)?;
        let user_id = match data {
            SessionData::Staff { staff_id } => *staff_id,
            // Client sessions are ticket-scoped; user_id (a staff fk default 0)
            // is left 0 — the realm/identity lives in the JSON payload.
            SessionData::Client { .. } => 0,
        };
        sqlx::query(
            "INSERT INTO session (session_id, session_data, session_expire, user_id) \
             VALUES ($1, $2, now() + ($3 || ' seconds')::interval, $4)",
        )
        .bind(&id)
        .bind(&payload)
        .bind(SESSION_TTL_SECS.to_string())
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(Session {
            id,
            data: data.clone(),
        })
    }

    /// Load a session by id **iff** it exists and has not expired.
    ///
    /// A row whose `session_expire` is in the past (or whose payload no longer
    /// parses) is treated as absent — the caller emits a 401. Expired rows are
    /// not auto-deleted here (a later sweep/cron handles GC).
    ///
    /// @implements FS-002.14: TTL enforcement on lookup (expired ⇒ invalid).
    pub async fn load(&self, id: &str) -> Result<Option<Session>, SessionError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT session_data FROM session \
             WHERE session_id = $1 AND (session_expire IS NULL OR session_expire > now())",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some((payload,)) => {
                let data: SessionData = serde_json::from_str(&payload)?;
                Ok(Some(Session {
                    id: id.to_string(),
                    data,
                }))
            }
            None => Ok(None),
        }
    }

    /// Delete a session by id (logout). A no-op when the id is unknown.
    pub async fn destroy(&self, id: &str) -> Result<(), SessionError> {
        sqlx::query("DELETE FROM session WHERE session_id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Load the permission flags for a staff member's group.
    ///
    /// Returns `None` when the staff id is unknown (e.g. a stale session whose
    /// account was removed). Used by the permission gate to evaluate a named
    /// permission against the staff member's group.
    ///
    /// @implements BS-001: group-permission lookup backing the permission gate.
    pub async fn staff_group_permissions(
        &self,
        staff_id: i32,
    ) -> Result<Option<crate::permission::GroupPermissions>, SessionError> {
        let row: Option<(bool, bool, bool)> = sqlx::query_as(
            "SELECT g.group_enabled, g.can_create_tickets, g.can_post_reply \
             FROM staff s JOIN groups g ON g.group_id = s.group_id \
             WHERE s.staff_id = $1 AND s.isactive = true",
        )
        .bind(staff_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(group_enabled, can_create_tickets, can_post_reply)| {
                crate::permission::GroupPermissions {
                    group_enabled,
                    can_create_tickets,
                    can_post_reply,
                }
            },
        ))
    }

    /// Insert a session row with an explicit absolute expiry — **test support**
    /// for asserting TTL handling (e.g. seed a row already past its 86400s
    /// window and confirm [`load`] treats it as expired).
    ///
    /// `expires_in` may be negative to produce an already-expired row.
    #[doc(hidden)]
    pub async fn create_with_expiry(
        &self,
        data: &SessionData,
        expires_in: Duration,
        negative: bool,
    ) -> Result<String, SessionError> {
        let id = new_session_id();
        let payload = serde_json::to_string(data)?;
        let secs = expires_in.as_secs() as i64;
        let secs = if negative { -secs } else { secs };
        sqlx::query(
            "INSERT INTO session (session_id, session_data, session_expire, user_id) \
             VALUES ($1, $2, now() + ($3 || ' seconds')::interval, 0)",
        )
        .bind(&id)
        .bind(&payload)
        .bind(secs.to_string())
        .execute(&self.pool)
        .await?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_ids_are_unique_and_long() {
        let a = new_session_id();
        let b = new_session_id();
        assert_ne!(a, b, "fresh ids must differ (fixation defence)");
        assert_eq!(a.len(), 64, "256-bit hex id");
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn session_data_round_trips_through_json_for_both_realms() {
        let staff = SessionData::Staff { staff_id: 7 };
        let json = serde_json::to_string(&staff).unwrap();
        assert!(json.contains("\"realm\":\"staff\""));
        assert_eq!(serde_json::from_str::<SessionData>(&json).unwrap(), staff);
        assert_eq!(staff.realm(), Realm::Staff);

        let client = SessionData::Client {
            ticket_number: 123456,
            email: "user@example.com".into(),
        };
        let json = serde_json::to_string(&client).unwrap();
        assert!(json.contains("\"realm\":\"client\""));
        assert_eq!(serde_json::from_str::<SessionData>(&json).unwrap(), client);
        assert_eq!(client.realm(), Realm::Client);
    }

    #[test]
    fn ttl_constant_is_86400() {
        assert_eq!(SESSION_TTL_SECS, 86_400);
    }
}
