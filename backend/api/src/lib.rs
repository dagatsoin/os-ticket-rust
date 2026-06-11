//! `api` — Axum HTTP server for the osTicket modernisation backend.
//!
//! Exposes [`app`] (the router builder, used by both the binary and the
//! integration tests) plus config, state, and the health handler. Owns the
//! shared error-envelope contract surfacing (404 fallback) per ROADMAP M1
//! Decision 4.

pub mod config;
pub mod health;
pub mod state;

use axum::routing::get;
use axum::Router;
use http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use ost_core::ApiError;

pub use config::Config;
pub use state::AppState;

/// Fallback handler for unmatched routes — emits the shared error envelope with
/// a 404 status.
///
/// @implements ROADMAP M1 Decision 4 — 404 not found in the shared envelope
/// (TS-M1-A1 AC-4).
async fn not_found() -> ApiError {
    ApiError::not_found("Not found")
}

/// Build the application router for a given [`AppState`] and CORS origin.
///
/// Routes:
/// - `GET /api/health` — DB reachability probe.
/// - everything else under the fallback ⇒ 404 error envelope.
///
/// Layers: permissive-for-one-origin CORS (the Vite dev origin) and request
/// tracing.
pub fn app(state: AppState, frontend_origin: &str) -> Router {
    let cors = build_cors(frontend_origin);

    Router::new()
        .route("/api/health", get(health::health))
        .fallback(not_found)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// CORS layer allowing the configured frontend origin. Falls back to a
/// permissive-method/header policy for that single origin.
///
/// @implements TS-M1-A1 — CORS for the Vite origin (`http://localhost:3702`).
fn build_cors(frontend_origin: &str) -> CorsLayer {
    let layer = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(tower_http::cors::Any)
        .allow_credentials(false);

    match HeaderValue::from_str(frontend_origin) {
        Ok(origin) => layer.allow_origin(origin),
        // An invalid configured origin must not crash the server; log and fall
        // back to denying cross-origin (no allowed origins).
        Err(_) => {
            tracing::warn!(origin = frontend_origin, "invalid CORS origin; disabling CORS");
            layer
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn health_returns_200_with_db_down_when_no_pool() {
        let router = app(AppState::without_db(), "http://localhost:3702");
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["status"], "ok");
        assert_eq!(json["db"], "down");
    }

    #[tokio::test]
    async fn unknown_route_returns_404_error_envelope() {
        let router = app(AppState::without_db(), "http://localhost:3702");
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/api/does-not-exist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let json = body_json(response).await;
        // Shared envelope shape: { "error": { "message", "fields" } }.
        assert!(json["error"].is_object());
        assert!(json["error"]["message"].is_string());
        assert!(json["error"]["fields"].is_object());
    }

    #[tokio::test]
    async fn cors_preflight_allows_configured_frontend_origin() {
        let router = app(AppState::without_db(), "http://localhost:3702");
        let response = router
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/api/health")
                    .header("Origin", "http://localhost:3702")
                    .header("Access-Control-Request-Method", "GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let allow_origin = response
            .headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok());
        assert_eq!(allow_origin, Some("http://localhost:3702"));
    }
}
