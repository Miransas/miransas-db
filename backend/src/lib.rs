use axum::{
    http::{HeaderValue, Method},
    middleware::from_fn_with_state,
    routing::{delete, get, post},
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
        .route(
            "/projects",
            get(handlers::projects::list_projects).post(handlers::projects::create_project),
        )
        .route(
            "/projects/:id",
            get(handlers::projects::get_project)
                .put(handlers::projects::update_project)
                .delete(handlers::projects::delete_project),
        )
        .route("/projects/:id/tables", get(handlers::projects::list_tables))
        .route(
            "/projects/:id/tables/:table",
            get(handlers::projects::get_table_data),
        )
        .route(
            "/projects/:id/query",
            post(handlers::projects::execute_query),
        )
        .route(
            "/projects/:id/tables/:table/:row_id",
            delete(handlers::projects::delete_row),
        )
        .route(
            "/projects/:id/connection",
            get(handlers::projects::get_connection),
        )
        .route(
            "/projects/:id/reset-password",
            post(handlers::projects::reset_project_password),
        )
        // ── User management ────────────────────────────────────────────────
        .route(
            "/projects/:id/user-config",
            get(handlers::users::get_config).put(handlers::users::put_config),
        )
        .route("/projects/:id/users", get(handlers::users::list_users))
        .route(
            "/projects/:id/users/export",
            get(handlers::users::export_users),
        )
        .route(
            "/projects/:id/users/:user_id",
            get(handlers::users::get_user).delete(handlers::users::delete_user),
        )
        .route(
            "/projects/:id/users/:user_id/ban",
            post(handlers::users::ban_user),
        )
        .route(
            "/projects/:id/users/:user_id/unban",
            post(handlers::users::unban_user),
        )
        .route(
            "/projects/:id/users/:user_id/password",
            post(handlers::users::reset_password),
        )
        // ── Secrets ────────────────────────────────────────────────────────
        .route(
            "/secrets",
            get(handlers::secrets::list_secrets).post(handlers::secrets::create_secret),
        )
        .route("/secrets/:id/reveal", get(handlers::secrets::reveal_secret))
        .route("/secrets/:id", delete(handlers::secrets::delete_secret))
        // ── Saved queries ──────────────────────────────────────────────────
        .route(
            "/projects/:project_id/saved-queries",
            get(handlers::saved_queries::list_saved_queries)
                .post(handlers::saved_queries::create_saved_query),
        )
        .route(
            "/saved-queries/:id",
            get(handlers::saved_queries::get_saved_query)
                .put(handlers::saved_queries::update_saved_query)
                .delete(handlers::saved_queries::delete_saved_query),
        )
        // ── Query history ──────────────────────────────────────────────────
        .route(
            "/projects/:project_id/query-history",
            get(handlers::query_log::list_history),
        )
        // ── Schema editor ──────────────────────────────────────────────────
        .route(
            "/projects/:project_id/schema/tables",
            post(handlers::schema::create_table),
        )
        .route(
            "/projects/:project_id/schema/tables/:table",
            get(handlers::schema::get_table_structure)
                .delete(handlers::schema::drop_table),
        )
        .route(
            "/projects/:project_id/schema/tables/:table/rename",
            post(handlers::schema::rename_table),
        )
        .route(
            "/projects/:project_id/schema/tables/:table/columns",
            post(handlers::schema::add_column),
        )
        .route(
            "/projects/:project_id/schema/tables/:table/columns/:column",
            delete(handlers::schema::drop_column),
        )
        .route(
            "/projects/:project_id/schema/tables/:table/columns/:column/rename",
            post(handlers::schema::rename_column),
        )
        .route(
            "/projects/:project_id/schema/tables/:table/columns/:column/type",
            post(handlers::schema::alter_column_type),
        )
        .route(
            "/projects/:project_id/schema/tables/:table/foreign-keys",
            post(handlers::schema::add_foreign_key),
        )
        .route(
            "/projects/:project_id/schema/tables/:table/check-constraints",
            post(handlers::schema::add_check_constraint),
        )
        .route(
            "/projects/:project_id/schema/tables/:table/constraints/:constraint_name",
            delete(handlers::schema::drop_constraint),
        )
        .route(
            "/projects/:project_id/schema/tables/:table/indexes",
            post(handlers::schema::create_index),
        )
        .route(
            "/projects/:project_id/schema/indexes/:index_name",
            delete(handlers::schema::drop_index),
        )
        // ── Audit logs ─────────────────────────────────────────────────────
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
        .route("/auth/login", post(handlers::auth::login))
        .nest("/api", api_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
