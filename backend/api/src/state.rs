//! Shared application state passed to every handler.

use std::sync::Arc;

use sqlx::postgres::PgPool;

use ost_core::session::SessionStore;
use ost_core::{BlobStore, Mailer, SmtpConfig, SmtpMailer, StubMailer};

use crate::config::AppEnv;

/// Resolve the workspace root for the default blob location: the binary runs
/// from the workspace root (`cargo run`/the deployed CWD), so the current dir is
/// used. `BlobStore::from_env` honours `BLOB_ROOT` over this default (§1).
fn default_blob_store() -> BlobStore {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    BlobStore::from_env(cwd)
}

/// The selected mailer transport plus the stub recorder backing the dev mailbox.
///
/// **DEVIATION D3 (§D3):** the *active* transport is the real [`SmtpMailer`] when
/// `SMTP_HOST` is set, otherwise the recording [`StubMailer`]. The `recorder` is
/// always the stub store `GET /api/dev/mailbox` reads — when SMTP is active it
/// stays empty (mail goes to the real relay, not the dev mailbox); when SMTP is
/// inactive `active` and `recorder` are the **same** stub (recorded intents show
/// in the dev mailbox, the M1 behaviour).
#[derive(Clone)]
pub struct MailerHandle {
    /// The transport that actually sends/records (`Arc<dyn Mailer>`).
    pub active: Arc<dyn Mailer>,
    /// The stub recorder backing `GET /api/dev/mailbox` (empty when SMTP active).
    pub recorder: Arc<StubMailer>,
}

impl MailerHandle {
    /// The M1 stub-only handle: `active` and `recorder` are the same stub.
    pub fn stub() -> Self {
        let stub = Arc::new(StubMailer::new());
        Self {
            active: stub.clone(),
            recorder: stub,
        }
    }

    /// Select the transport from the environment (§D3): `SMTP_HOST` set ⇒ the
    /// real [`SmtpMailer`]; otherwise the stub. A misconfigured SMTP transport
    /// logs and falls back to the stub so the server still boots.
    ///
    /// @implements FS-040.12 / §D3: startup transport selection.
    pub fn from_env() -> Self {
        match SmtpConfig::from_env() {
            Some(cfg) => match SmtpMailer::new(cfg.clone()) {
                Ok(smtp) => {
                    tracing::info!(host = %cfg.host, port = cfg.port, "SMTP mailer active");
                    Self {
                        active: Arc::new(smtp),
                        recorder: Arc::new(StubMailer::new()),
                    }
                }
                Err(err) => {
                    tracing::error!(error = %err, "SMTP mailer init failed; falling back to stub");
                    Self::stub()
                }
            },
            None => {
                tracing::info!("SMTP_HOST unset; stub mailer active (dev mailbox)");
                Self::stub()
            }
        }
    }

    /// Build a handle from an explicit active mailer + stub recorder (tests).
    pub fn from_parts(active: Arc<dyn Mailer>, recorder: Arc<StubMailer>) -> Self {
        Self { active, recorder }
    }
}

/// Application state. The pool is optional so the server boots (and the health
/// endpoint / error contract stay testable) even with no live DB.
#[derive(Clone)]
pub struct AppState {
    pub pool: Option<PgPool>,
    /// DB-backed session store (present iff a pool is configured). Owned by
    /// TS-M1-A4b; consumed by the realm gates and the login/logout routes.
    pub sessions: Option<SessionStore>,
    /// The selected mailer transport + the dev-mailbox recorder (TS-M1-A4b /
    /// TS-M2-E1, §D3). Shared via `Arc` so recorded sends survive the process
    /// lifetime (`GET /api/dev/mailbox`).
    pub mailer: MailerHandle,
    /// Deployment environment — drives cookie `Secure` + dev endpoint gating.
    pub app_env: AppEnv,
    /// Content-addressed blob store for attachments (TS-M2-A1, §1). Resolved from
    /// `BLOB_ROOT` (default `<cwd>/var/blobs`) at boot; shared by the upload
    /// (A3/A5) and download (B1) routes.
    pub store: BlobStore,
}

impl AppState {
    /// State with a connected pool (wires the DB-backed session store).
    ///
    /// The mailer is selected from the environment (§D3): SMTP when `SMTP_HOST`
    /// is set, otherwise the recording stub.
    pub fn with_pool(pool: PgPool) -> Self {
        Self {
            sessions: Some(SessionStore::new(pool.clone())),
            pool: Some(pool),
            mailer: MailerHandle::from_env(),
            app_env: AppEnv::Development,
            store: default_blob_store(),
        }
    }

    /// State with no DB pool (health reports `db: "down"`; no session store).
    pub fn without_db() -> Self {
        Self {
            pool: None,
            sessions: None,
            mailer: MailerHandle::from_env(),
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

    /// Override the mailer handle (chainable). Used by tests to inject a stub or
    /// an explicit SMTP transport regardless of the ambient environment.
    #[must_use]
    pub fn with_mailer(mut self, mailer: MailerHandle) -> Self {
        self.mailer = mailer;
        self
    }
}
