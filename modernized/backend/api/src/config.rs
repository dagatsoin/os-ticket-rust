//! Runtime configuration loaded from the environment (`.env` / `APP_*`).
//!
//! @implements TS-M1-A1 — config loading for the Axum skeleton.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Default bind port for the backend API (machine port convention, range 37xx).
pub const DEFAULT_PORT: u16 = 3701;

/// Default Vite frontend origin allowed by CORS.
pub const DEFAULT_FRONTEND_ORIGIN: &str = "http://localhost:3702";

/// Default deployment environment when `APP_ENV` is unset — **fail safe**:
/// absence defaults to production so a missing env var never exposes dev
/// endpoints (WARN-1). Dev/QA must set `APP_ENV=development` explicitly.
pub const DEFAULT_APP_ENV: &str = "production";

/// Deployment environment. Drives cookie `Secure` (off in dev) and whether the
/// dev mailbox endpoint is exposed (disabled in production).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEnv {
    /// Local development (cookies not `Secure`, dev endpoints enabled).
    Development,
    /// Production (cookies `Secure`, dev endpoints disabled).
    Production,
}

impl AppEnv {
    /// Parse from the `APP_ENV` string, failing **safe** (WARN-1): only the
    /// explicit values `development` / `test` (case-insensitive) enable the
    /// unauthenticated dev endpoints. Anything else — including an unset,
    /// empty, or misspelled value — resolves to [`AppEnv::Production`], so a
    /// missing env var in a production deploy can never open the dev surface.
    pub fn from_str_lossy(s: &str) -> Self {
        if s.eq_ignore_ascii_case("development") || s.eq_ignore_ascii_case("test") {
            AppEnv::Development
        } else {
            AppEnv::Production
        }
    }

    /// Whether session/CSRF cookies should carry the `Secure` attribute.
    pub fn cookies_secure(self) -> bool {
        matches!(self, AppEnv::Production)
    }

    /// Whether env-gated dev endpoints (e.g. `GET /api/dev/mailbox`) are exposed.
    pub fn dev_endpoints_enabled(self) -> bool {
        matches!(self, AppEnv::Development)
    }
}

/// Resolved server configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Postgres connection string (`DATABASE_URL`). `None` when unset — the
    /// server still boots and reports `db: "down"` so the health endpoint and
    /// CORS/error contract remain testable without a live DB.
    pub database_url: Option<String>,
    /// Address the HTTP server binds to.
    pub bind_addr: SocketAddr,
    /// Origin allowed by CORS (the Vite dev origin).
    pub frontend_origin: String,
    /// Deployment environment (drives cookie `Secure` + dev endpoint gating).
    pub app_env: AppEnv,
}

impl Config {
    /// Load configuration from process environment variables.
    ///
    /// - `DATABASE_URL` — Postgres DSN (optional; absence ⇒ `db: "down"`).
    /// - `APP_PORT` / `PORT` — bind port (default [`DEFAULT_PORT`] = 3701).
    /// - `APP_HOST` — bind host (default `0.0.0.0`).
    /// - `APP_FRONTEND_ORIGIN` — CORS origin (default [`DEFAULT_FRONTEND_ORIGIN`]).
    pub fn from_env() -> Self {
        let database_url = std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty());

        let port = std::env::var("APP_PORT")
            .or_else(|_| std::env::var("PORT"))
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(DEFAULT_PORT);

        let host = std::env::var("APP_HOST")
            .ok()
            .and_then(|s| s.parse::<IpAddr>().ok())
            .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

        let frontend_origin = std::env::var("APP_FRONTEND_ORIGIN")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_FRONTEND_ORIGIN.to_string());

        let app_env = std::env::var("APP_ENV")
            .ok()
            .filter(|s| !s.is_empty())
            .map(|s| AppEnv::from_str_lossy(&s))
            .unwrap_or_else(|| AppEnv::from_str_lossy(DEFAULT_APP_ENV));

        Self {
            database_url,
            bind_addr: SocketAddr::new(host, port),
            frontend_origin,
            app_env,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_env_fails_safe_to_production() {
        // WARN-1: only explicit development/test enable dev endpoints.
        assert_eq!(AppEnv::from_str_lossy("development"), AppEnv::Development);
        assert_eq!(AppEnv::from_str_lossy("DEVELOPMENT"), AppEnv::Development);
        assert_eq!(AppEnv::from_str_lossy("test"), AppEnv::Development);
        // Everything else — production, unset-default, empty, misspelled — is
        // production, so dev endpoints stay closed.
        assert_eq!(AppEnv::from_str_lossy("production"), AppEnv::Production);
        assert_eq!(AppEnv::from_str_lossy(""), AppEnv::Production);
        assert_eq!(AppEnv::from_str_lossy("prod"), AppEnv::Production);
        assert_eq!(AppEnv::from_str_lossy("develop"), AppEnv::Production);
        assert_eq!(AppEnv::from_str_lossy("dev"), AppEnv::Production);
        // The unset default resolves to production too.
        assert_eq!(AppEnv::from_str_lossy(DEFAULT_APP_ENV), AppEnv::Production);
    }

    #[test]
    fn dev_endpoints_enabled_only_in_development() {
        assert!(AppEnv::Development.dev_endpoints_enabled());
        assert!(!AppEnv::Production.dev_endpoints_enabled());
        // Cookies are Secure only in production.
        assert!(AppEnv::Production.cookies_secure());
        assert!(!AppEnv::Development.cookies_secure());
    }
}
