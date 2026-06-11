//! Binary entry point for the osTicket modernisation backend API.
//!
//! Loads `.env`, initialises tracing, connects to Postgres (non-fatal — a
//! db-down boot still serves `/api/health` with `db: "down"`), and binds the
//! Axum server to the configured port (default 3701).

use api::{app, AppState, Config};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env if present (ignore absence — env vars may come from the shell).
    let _ = dotenvy::dotenv();

    init_tracing();

    let config = Config::from_env();

    // Connect to the DB if a URL is configured. A failure here is non-fatal:
    // the server still boots and `/api/health` reports `db: "down"` so the
    // frontend smoke check and error contract stay observable.
    let state = match &config.database_url {
        Some(url) => match db::connect(url).await {
            Ok(pool) => {
                tracing::info!("connected to database");
                AppState::with_pool(pool)
            }
            Err(err) => {
                tracing::warn!(error = %err, "database unavailable at startup; serving with db: down");
                AppState::without_db()
            }
        },
        None => {
            tracing::warn!("DATABASE_URL not set; serving with db: down");
            AppState::without_db()
        }
    };

    let router = app(state, &config.frontend_origin);

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, frontend_origin = %config.frontend_origin, "api listening");

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Initialise structured logging. Honours `RUST_LOG`; defaults to `info` for the
/// app crates.
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,api=debug,db=info,tower_http=info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

/// Wait for Ctrl-C for graceful shutdown.
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
