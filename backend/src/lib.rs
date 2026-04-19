use axum::{
    http::{HeaderValue, Method},
    middleware::from_fn_with_state,
    routing::{get, post},
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
        // Projects
        .route(
            "/projects",
            get(handlers::projects::list_projects).post(handlers::projects::create_project),
        )
        // Databases — list / create
        .route(
            "/databases",
            get(handlers::databases::list_databases).post(handlers::databases::create_database),
        )
        // Database exploration
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
        // Secrets
        .route(
            "/secrets",
            get(handlers::secrets::list_secrets).post(handlers::secrets::create_secret),
        )
        // Admin
        .route("/admin/summary", get(handlers::admin::summary))
        // Every /api route requires a valid JWT
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
