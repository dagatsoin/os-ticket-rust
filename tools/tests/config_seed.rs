//! Integration tests for the TS-M2-A2 attachment + helpdesk config-key seed.
//!
//! DB-backed; skipped (pass with a log line) when `TEST_DATABASE_URL` is unset.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p tools config_seed
//! ```
//!
//! @implements FS-022.13 / ROADMAP §6: the four M2 config keys seed idempotently
//!   with the pinned defaults (TS-M2-A2 AC-5).

use sqlx::postgres::PgPool;
use tools::{
    CFG_ALLOWED_FILETYPES, CFG_ALLOW_ATTACHMENTS, CFG_HELPDESK_URL, CFG_MAX_FILE_SIZE,
    DEFAULT_ALLOWED_FILETYPES, DEFAULT_ALLOW_ATTACHMENTS, DEFAULT_HELPDESK_URL,
    DEFAULT_MAX_FILE_SIZE,
};

/// The AC-named module so `cargo test -p tools config_seed::idempotent` selects
/// exactly the AC-5 idempotency case.
mod config_seed {
    use super::*;

    fn test_db_url() -> Option<String> {
        std::env::var("TEST_DATABASE_URL").ok()
    }

    async fn fresh_pool() -> Option<PgPool> {
        let url = test_db_url()?;
        let pool = db::connect(&url).await.expect("connect");
        db::migrate(&pool).await.expect("migrate");
        // Isolate: clear the four M2 keys so a fresh seed exercises the INSERT.
        sqlx::query(
            "DELETE FROM config WHERE key IN \
             ('allow_attachments','allowed_filetypes','max_file_size','helpdesk_url')",
        )
        .execute(&pool)
        .await
        .expect("clear m2 keys");
        Some(pool)
    }

    async fn value_of(pool: &PgPool, key: &str) -> Option<String> {
        sqlx::query_scalar("SELECT value FROM config WHERE key = $1")
            .bind(key)
            .fetch_optional(pool)
            .await
            .unwrap()
    }

    async fn row_count(pool: &PgPool, key: &str) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM config WHERE key = $1")
            .bind(key)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    /// AC-5: the four keys seed with the pinned defaults; a second run leaves
    /// exactly one row per key (idempotent).
    #[tokio::test]
    async fn idempotent() {
        let Some(pool) = fresh_pool().await else {
            eprintln!("TEST_DATABASE_URL unset — skipping config_seed::idempotent");
            return;
        };

        // First seed → pinned values present.
        tools::seed(&pool).await.expect("first seed");
        assert_eq!(
            value_of(&pool, CFG_ALLOW_ATTACHMENTS).await.as_deref(),
            Some(DEFAULT_ALLOW_ATTACHMENTS)
        );
        assert_eq!(
            value_of(&pool, CFG_ALLOWED_FILETYPES).await.as_deref(),
            Some(DEFAULT_ALLOWED_FILETYPES)
        );
        assert_eq!(
            value_of(&pool, CFG_MAX_FILE_SIZE).await.as_deref(),
            Some(DEFAULT_MAX_FILE_SIZE)
        );
        assert_eq!(
            value_of(&pool, CFG_HELPDESK_URL).await.as_deref(),
            Some(DEFAULT_HELPDESK_URL)
        );

        // Second seed → still exactly one row per key (no duplicates, no error).
        tools::seed(&pool).await.expect("second seed");
        for key in [
            CFG_ALLOW_ATTACHMENTS,
            CFG_ALLOWED_FILETYPES,
            CFG_MAX_FILE_SIZE,
            CFG_HELPDESK_URL,
        ] {
            assert_eq!(row_count(&pool, key).await, 1, "exactly one row for `{key}`");
        }

        // An admin-tuned value survives a re-seed (ON CONFLICT DO NOTHING, not
        // overwrite). Merged into this test so the two cases never interleave on
        // the shared single-row config keys.
        sqlx::query("UPDATE config SET value = '.pdf' WHERE key = $1")
            .bind(CFG_ALLOWED_FILETYPES)
            .execute(&pool)
            .await
            .unwrap();
        tools::seed(&pool).await.expect("third seed");
        assert_eq!(
            value_of(&pool, CFG_ALLOWED_FILETYPES).await.as_deref(),
            Some(".pdf"),
            "re-seed must preserve an admin-set value (ON CONFLICT DO NOTHING)"
        );
    }
}
