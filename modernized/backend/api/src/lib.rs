//! `api` — Axum HTTP server for the osTicket modernisation backend.
//!
//! Exposes [`app`] (the router builder, used by both the binary and the
//! integration tests) plus config, state, and the health handler. Owns the
//! shared error-envelope contract surfacing (404 fallback) per ROADMAP M1
//! Decision 4.

pub mod admin_canned;
pub mod admin_groups;
pub mod admin_staff;
pub mod attachments;
pub mod auth;
pub mod canned;
pub mod client;
pub mod config;
pub mod config_keys;
pub mod departments;
pub mod dev;
pub mod downloads;
pub mod email_wiring;
pub mod faq_categories;
pub mod health;
pub mod help_topics;
pub mod logs;
pub mod pages;
pub mod profile;
pub mod settings;
pub mod sla;
pub mod staff;
pub mod state;
pub mod teams;
pub mod tickets;

use axum::routing::{delete, get, post, put};
use axum::Router;
use http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use ost_core::ApiError;

pub use config::{AppEnv, Config};
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
        // Public create-ticket — unauthenticated + CSRF-exempt (Decision 2).
        .route("/api/tickets", post(tickets::create_public_ticket))
        // Public thread view — unauthenticated, M/R only (BS-021.10: notes excluded).
        .route("/api/tickets/:id/thread", get(tickets::get_public_thread))
        // Auth — both login POSTs are unauthenticated + CSRF-exempt (Decision 2).
        .route("/api/staff/login", post(auth::routes::staff_login))
        .route("/api/staff/logout", post(auth::routes::staff_logout))
        // Staff realm — gated by the StaffSession extractor (401 without it).
        .route("/api/staff/me", get(staff::me))
        // Transfer/assign/edit reference lists — any staff session (NOT admin),
        // read-only id+name only. Registered before the `:id` ticket routes so
        // the static `ticket-options` segment cannot be shadowed.
        .route("/api/staff/ticket-options", get(staff::ticket_options))
        // Admin settings (TS-M4-A1) — per-route admin gate (require_admin), not
        // blanket middleware; PUT additionally CSRF-enforced (StaffCsrf).
        .route(
            "/api/staff/admin/settings",
            get(settings::get_settings).put(settings::put_settings),
        )
        // Admin staff management (TS-M4-B1) — admin-gated; mutating routes CSRF.
        // `mass` is registered before `:id` (static wins in matchit anyway).
        .route(
            "/api/staff/admin/staff",
            get(admin_staff::list_staff).post(admin_staff::create_staff),
        )
        .route("/api/staff/admin/staff/mass", post(admin_staff::mass_staff))
        .route("/api/staff/admin/staff/:id", put(admin_staff::update_staff))
        .route("/api/staff/admin/staff/:id/teams", post(admin_staff::add_team))
        .route(
            "/api/staff/admin/staff/:id/teams/:teamId",
            delete(admin_staff::remove_team),
        )
        // Admin group management (TS-M4-B3) — admin-gated; mutating routes CSRF.
        .route(
            "/api/staff/admin/groups",
            get(admin_groups::list_groups).post(admin_groups::create_group),
        )
        .route("/api/staff/admin/groups/mass", post(admin_groups::mass_groups))
        .route(
            "/api/staff/admin/groups/:id",
            get(admin_groups::get_group).put(admin_groups::update_group),
        )
        // Admin SLA management (TS-M4-D1) — admin-gated; mutating routes CSRF.
        // `mass` before `:id` (static wins in matchit anyway).
        .route(
            "/api/staff/admin/sla",
            get(sla::list_sla).post(sla::create_sla),
        )
        .route("/api/staff/admin/sla/mass", post(sla::mass_sla))
        .route(
            "/api/staff/admin/sla/:id",
            put(sla::update_sla).delete(sla::delete_sla),
        )
        // Read-only priority set (TS-M4-D1 / BS-032.13) — admin-gated, no CRUD.
        .route("/api/staff/admin/priorities", get(sla::list_priorities))
        // Admin department management (TS-M4-C1) — admin-gated; mutating routes CSRF.
        // static `form-options` / `mass` before `:id` (static wins in matchit anyway).
        .route(
            "/api/staff/admin/departments",
            get(departments::list_departments).post(departments::create_department),
        )
        .route(
            "/api/staff/admin/departments/form-options",
            get(departments::form_options),
        )
        .route(
            "/api/staff/admin/departments/mass",
            post(departments::mass_departments),
        )
        .route(
            "/api/staff/admin/departments/:id",
            get(departments::get_department)
                .put(departments::update_department)
                .delete(departments::delete_department),
        )
        // Admin team management (TS-M4-C3) — admin-gated; mutating routes CSRF.
        .route(
            "/api/staff/admin/teams",
            get(teams::list_teams).post(teams::create_team),
        )
        .route("/api/staff/admin/teams/mass", post(teams::mass_teams))
        .route(
            "/api/staff/admin/teams/:id",
            get(teams::get_team)
                .put(teams::update_team)
                .delete(teams::delete_team),
        )
        // Admin help-topic management (TS-M4-C5) — admin-gated; mutating routes CSRF.
        // static `form-options` / `mass` before `:id` (static wins in matchit anyway).
        .route(
            "/api/staff/admin/help-topics",
            get(help_topics::list_topics).post(help_topics::create_topic),
        )
        .route(
            "/api/staff/admin/help-topics/form-options",
            get(help_topics::form_options),
        )
        .route(
            "/api/staff/admin/help-topics/mass",
            post(help_topics::mass_topics),
        )
        .route(
            "/api/staff/admin/help-topics/:id",
            get(help_topics::get_topic)
                .put(help_topics::update_topic)
                .delete(help_topics::delete_topic),
        )
        // FAQ-category management (TS-M4-E1) — NOT /admin; gated by
        // `can_manage_faq` (admin OR flag) on every handler; mutating routes CSRF.
        .route(
            "/api/staff/faq-categories",
            get(faq_categories::list_categories).post(faq_categories::create_category),
        )
        .route(
            "/api/staff/faq-categories/mass",
            post(faq_categories::mass_categories),
        )
        .route(
            "/api/staff/faq-categories/:id",
            get(faq_categories::get_category)
                .put(faq_categories::update_category)
                .delete(faq_categories::delete_category),
        )
        // Admin site-page management (TS-M4-F1) — admin-gated; mutating CSRF.
        .route(
            "/api/staff/admin/pages",
            get(pages::list_pages).post(pages::create_page),
        )
        .route("/api/staff/admin/pages/mass", post(pages::mass_pages))
        .route("/api/staff/admin/pages/:id", put(pages::update_page))
        // Content / config read endpoints (TS-M4-F1) — admin-gated.
        .route(
            "/api/staff/admin/content/ticket_variables",
            get(pages::ticket_variables),
        )
        .route("/api/config/scp", get(pages::config_scp))
        // Public site page by slug (TS-M4-F1) — UNAUTHENTICATED.
        .route("/api/pages/:slug", get(pages::public_page))
        // Admin system-log viewer (TS-M4-G2) — admin-gated; delete CSRF.
        // `delete` before `:id` (static wins in matchit anyway).
        .route("/api/staff/admin/logs", get(logs::list_logs))
        .route("/api/staff/admin/logs/delete", post(logs::delete_logs))
        .route("/api/staff/admin/logs/:id", get(logs::get_log))
        // Canned-response CRUD (TS-M4-H1) — premade-gated; mutating routes CSRF.
        // static `dept-options` / `mass` before `:id` (static wins anyway).
        .route(
            "/api/staff/canned-responses",
            get(admin_canned::list_canned).post(admin_canned::create_canned),
        )
        .route(
            "/api/staff/canned-responses/dept-options",
            get(admin_canned::dept_options),
        )
        .route("/api/staff/canned-responses/mass", post(admin_canned::mass_canned))
        .route(
            "/api/staff/canned-responses/:id",
            get(admin_canned::get_canned)
                .put(admin_canned::update_canned)
                .delete(admin_canned::delete_canned),
        )
        // Own-profile + directory (TS-M4-B5) — staff realm (directory is NOT admin).
        .route(
            "/api/staff/profile",
            get(profile::get_profile).put(profile::update_profile),
        )
        .route("/api/staff/profile/password", put(profile::change_password))
        .route("/api/staff/directory", get(profile::directory))
        .route("/api/staff/tickets", get(staff::list_tickets))
        // Stats, sort-prefs, and bulk must be registered BEFORE the :id route to avoid path conflicts.
        .route("/api/staff/tickets/stats", get(staff::get_stats))
        .route("/api/staff/tickets/sort-prefs", get(staff::get_sort_prefs))
        // Bulk actions (TS-M3-G1) — staff realm + CSRF.
        .route("/api/staff/tickets/bulk", post(staff::bulk_action_route))
        .route("/api/staff/tickets/:id", get(staff::ticket_detail))
        .route("/api/staff/tickets/:id/reply", post(staff::reply))
        // Workflow actions (TS-M3-C1/C2/C3) — staff realm + CSRF.
        .route("/api/staff/tickets/:id/close", post(staff::close_ticket_route))
        .route("/api/staff/tickets/:id/reopen", post(staff::reopen_ticket_route))
        .route("/api/staff/tickets/:id/claim", post(staff::claim_ticket_route))
        .route("/api/staff/tickets/:id/assign", post(staff::assign_ticket_route))
        .route("/api/staff/tickets/:id/transfer", post(staff::transfer_ticket_route))
        // Internal notes (TS-M3-E1/E2) — staff realm + CSRF.
        .route("/api/staff/tickets/:id/note", post(staff::post_note_route))
        .route("/api/staff/tickets/:id/thread", get(staff::get_thread))
        // Delete ticket (TS-M3-I2) — staff realm + CSRF.
        .route("/api/staff/tickets/:id", delete(staff::delete_ticket_route))
        // Update ticket (TS-M3-I1) — staff realm + CSRF.
        .route("/api/staff/tickets/:id", put(staff::update_ticket_route))
        // Ticket locking (TS-M3-I3) — staff realm + CSRF.
        .route("/api/staff/tickets/:id/lock", post(staff::acquire_lock_route))
        .route("/api/staff/tickets/:id/lock", delete(staff::release_lock_route))
        // Canned-response fetch (staff realm, read-only) — TS-M2-D2.
        .route("/api/staff/tickets/:id/canned", get(canned::list_canned))
        .route(
            "/api/staff/tickets/:id/canned/:cannedId",
            get(canned::get_canned),
        )
        // Staff attachment download (parent-ticket gate, §8) — TS-M2-B1.
        .route(
            "/api/staff/tickets/:ticketId/attachments/:attachmentId",
            get(downloads::staff_download),
        )
        .route("/api/client/login", post(auth::routes::client_login))
        .route("/api/client/logout", post(auth::routes::client_logout))
        // Client realm — gated by the ClientSession extractor (401 without it).
        .route("/api/client/ticket", get(client::ticket))
        // Client attachment download (session-bound, NO ticketId param, §8) — B1.
        .route(
            "/api/client/ticket/attachments/:attachmentId",
            get(downloads::client_download),
        )
        // Env-gated dev endpoints (404 in production).
        .route("/api/dev/mailbox", get(dev::mailbox))
        .route("/api/dev/seed-ticket", post(dev::seed_ticket))
        .route("/api/dev/seed-tickets", post(dev::seed_tickets))
        .route("/api/dev/seed-staff", post(dev::seed_staff))
        // TS-M3-C: workflow testing dev endpoints.
        .route("/api/dev/set-group-perm", post(dev::set_group_perm))
        .route("/api/dev/seed-dept", post(dev::seed_dept))
        .route("/api/dev/set-dept-sla", post(dev::set_dept_sla))
        .route("/api/dev/seed-sla", post(dev::seed_sla))
        // TS-M3-E: note/state testing dev endpoints.
        .route("/api/dev/mark-overdue/:id", post(dev::mark_overdue))
        // TS-M3-F1/F2: search testing dev endpoints.
        .route("/api/dev/reset-tickets", post(dev::reset_tickets))
        .route("/api/dev/seed-search-ticket", post(dev::seed_search_ticket))
        .route("/api/dev/seed-topic", post(dev::seed_topic))
        // TS-M3-I3: lock testing dev endpoints.
        .route("/api/dev/seed-config", post(dev::seed_config))
        // Toggle config values for browser testing (show_answered_tickets, show_assigned_tickets).
        .route("/api/dev/toggle-config", post(dev::toggle_config))
        // TS-M4-PREP: dev endpoints backing the M4 admin epics.
        .route("/api/dev/seed-log", post(dev::seed_log))
        .route("/api/dev/purge-logs", post(dev::purge_logs))
        .route("/api/dev/age-password", post(dev::age_password))
        .route("/api/dev/reset-staff", post(dev::reset_staff))
        .route("/api/dev/reset-config", post(dev::reset_config))
        // TS-M4-B3: reset non-seed groups for a clean group-list E2E.
        .route("/api/dev/reset-groups", post(dev::reset_groups))
        // TS-M4-B1/B3: bulk-seed staff for pagination ACs.
        .route("/api/dev/seed-staff-bulk", post(dev::seed_staff_bulk))
        // TS-M4-A1: seed a site page for the settings Pages/Landing AC.
        .route("/api/dev/seed-page", post(dev::seed_page))
        // TS-M4-D1/F1/H1: cheap reset paths for the SLA / pages / canned E2E.
        .route("/api/dev/reset-sla", post(dev::reset_sla))
        .route("/api/dev/reset-pages", post(dev::reset_pages))
        .route("/api/dev/reset-canned", post(dev::reset_canned))
        // TS-M4-C1/C3/C5/E1: reset paths for the departments / teams / help-topics /
        // faq-categories E2E (repoint dependents to seed baseline, delete non-seed).
        .route("/api/dev/reset-departments", post(dev::reset_departments))
        .route("/api/dev/reset-teams", post(dev::reset_teams))
        .route("/api/dev/reset-help-topics", post(dev::reset_help_topics))
        .route("/api/dev/reset-faq-categories", post(dev::reset_faq_categories))
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
    // Cookie-based auth requires `allow_credentials(true)`. The CORS spec forbids
    // pairing credentials with a wildcard origin/headers, so the origin is a
    // single exact value and the allowed headers are enumerated (including the
    // `X-CSRFToken` double-submit header — ROADMAP Decision 2).
    let layer = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            http::header::CONTENT_TYPE,
            http::header::ACCEPT,
            http::HeaderName::from_static("x-csrftoken"),
        ])
        .allow_credentials(true);

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
