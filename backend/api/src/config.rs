//! Runtime configuration loaded from the environment (`.env` / `APP_*`).
//!
//! @implements TS-M1-A1 — config loading for the Axum skeleton.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Default bind port for the backend API (machine port convention, range 37xx).
pub const DEFAULT_PORT: u16 = 3701;

/// Default Vite frontend origin allowed by CORS.
pub const DEFAULT_FRONTEND_ORIGIN: &str = "http://localhost:3702";

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

        Self {
            database_url,
            bind_addr: SocketAddr::new(host, port),
            frontend_origin,
        }
    }
}
