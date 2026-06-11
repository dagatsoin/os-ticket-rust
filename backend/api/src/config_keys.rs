//! Tiny helper to read a single `config` key/value at request time.
//!
//! The canned-response substitution (D2) and the D4 reply wiring read
//! `helpdesk_url` (§6) to back `%{url}`. This centralises the one-key lookup so
//! both paths share it (the multi-key attachment policy lives in
//! [`crate::attachments::load_upload_policy`]).

use sqlx::postgres::PgPool;

use ost_core::ApiError;

/// Read a single `config` value by key. `Ok(None)` when the key is absent.
///
/// @implements ROADMAP §6: read the `helpdesk_url` config key at request time.
pub async fn read_config(pool: &PgPool, key: &str) -> Result<Option<String>, ApiError> {
    sqlx::query_scalar::<_, String>("SELECT value FROM config WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal("Config lookup failed"))
}
