use axum::{
    http::{HeaderValue, Method},
    middleware::from_fn_with_state,
    routing::{delete, get, post, put},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub mod config;
pub mod db;
pub mod errors;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod services;
pub mod state;
pub mod utils;

use state::AppState;

pub fn build_router(state: AppState) -> Router {
    let cors = {
        let origin = state
            .config
            .cors_origin
            .parse::<HeaderValue>()
            .unwrap_or_else(|_| HeaderValue::from_static("http://localhost:3000"));

        CorsLayer::new()
            .allow_origin(origin)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
            ])
            .allow_credentials(true)
    };

    let api_routes = Router::new()
        // ── Projects ───────────────────────────────────────────────────────
        // GET  /api/projects
        // POST /api/projects          { name, connection_string, description?, repository_url? }
        .route(
            "/projects",
            get(handlers::projects::list_projects).post(handlers::projects::create_project),
        )
        // PUT    /api/projects/:id   { name?, description?, repository_url?, connection_string? }
        // DELETE /api/projects/:id
        .route(
            "/projects/:id",
            put(handlers::projects::update_project).delete(handlers::projects::delete_project),
        )
        // GET  /api/projects/:id/tables
        .route("/projects/:id/tables", get(handlers::projects::list_tables))
        // GET  /api/projects/:id/tables/:table?page=1&limit=50
        .route(
            "/projects/:id/tables/:table",
            get(handlers::projects::get_table_data),
        )
        // POST /api/projects/:id/query          { sql }
        .route(
            "/projects/:id/query",
            post(handlers::projects::execute_query),
        )
        // DELETE /api/projects/:id/tables/:table/:row_id?pk=id
        .route(
            "/projects/:id/tables/:table/:row_id",
            delete(handlers::projects::delete_row),
        )
        // ── Databases (connection-URL registry) ────────────────────────────
        .route(
            "/databases",
            get(handlers::databases::list_databases).post(handlers::databases::create_database),
        )
        .route(
            "/databases/:id/tables",
            get(handlers::databases::list_tables),
        )
        .route(
            "/databases/:id/tables/:table",
            get(handlers::databases::get_table_data),
        )
        .route(
            "/databases/:id/query",
            post(handlers::databases::execute_query),
        )
        // ── Secrets ────────────────────────────────────────────────────────
        .route(
            "/secrets",
            get(handlers::secrets::list_secrets).post(handlers::secrets::create_secret),
        )
        // ── Audit logs ─────────────────────────────────────────────────────
        // GET /api/audit-logs?page=1&limit=50&resource_type=...&resource_id=...
        .route("/audit-logs", get(handlers::audit::list_audit_logs))
        // ── Admin ──────────────────────────────────────────────────────────
        .route("/admin/summary", get(handlers::admin::summary))
        // Every /api/* route requires a valid JWT
        .route_layer(from_fn_with_state(
            state.clone(),
            middleware::auth::require_auth,
        ));

    Router::new()
        .route("/health", get(handlers::health::health))
        // POST /auth/login  { password } → { token, expires_in }
        .route("/auth/login", post(handlers::auth::login))
        .nest("/api", api_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
