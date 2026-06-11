//! Shared application state passed to every handler.

use std::sync::Arc;

use sqlx::postgres::PgPool;

use ost_core::session::SessionStore;
use ost_core::{BlobStore, StubMailer};

use crate::config::AppEnv;

/// Resolve the workspace root for the default blob location: the binary runs
/// from the workspace root (`cargo run`/the deployed CWD), so the current dir is
/// used. `BlobStore::from_env` honours `BLOB_ROOT` over this default (§1).
fn default_blob_store() -> BlobStore {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    BlobStore::from_env(cwd)
}

/// Application state. The pool is optional so the server boots (and the health
/// endpoint / error contract stay testable) even with no live DB.
#[derive(Clone)]
pub struct AppState {
    pub pool: Option<PgPool>,
    /// DB-backed session store (present iff a pool is configured). Owned by
    /// TS-M1-A4b; consumed by the realm gates and the login/logout routes.
    pub sessions: Option<SessionStore>,
    /// The stub mailer's recording store (TS-M1-A4b). Shared via `Arc` so the
    /// recorded sends survive for the process lifetime (QA reads them back via
    /// `GET /api/dev/mailbox`).
    pub mailer: Arc<StubMailer>,
    /// Deployment environment — drives cookie `Secure` + dev endpoint gating.
    pub app_env: AppEnv,
    /// Content-addressed blob store for attachments (TS-M2-A1, §1). Resolved from
    /// `BLOB_ROOT` (default `<cwd>/var/blobs`) at boot; shared by the upload
    /// (A3/A5) and download (B1) routes.
    pub store: BlobStore,
}

impl AppState {
    /// State with a connected pool (wires the DB-backed session store).
    pub fn with_pool(pool: PgPool) -> Self {
        Self {
            sessions: Some(SessionStore::new(pool.clone())),
            pool: Some(pool),
            mailer: Arc::new(StubMailer::new()),
            app_env: AppEnv::Development,
            store: default_blob_store(),
        }
    }

    /// State with no DB pool (health reports `db: "down"`; no session store).
    pub fn without_db() -> Self {
        Self {
            pool: None,
            sessions: None,
            mailer: Arc::new(StubMailer::new()),
            app_env: AppEnv::Development,
            store: default_blob_store(),
        }
    }

    /// Override the blob store (chainable). Used by integration tests to point
    /// the store at an isolated temp dir.
    #[must_use]
    pub fn with_store(mut self, store: BlobStore) -> Self {
        self.store = store;
        self
    }

    /// Override the deployment environment (chainable). Used at boot and in
    /// tests that exercise production gating.
    #[must_use]
    pub fn with_app_env(mut self, app_env: AppEnv) -> Self {
        self.app_env = app_env;
        self
    }
}
