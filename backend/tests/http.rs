use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use miransas_db::{build_router, config::Config, state::AppState};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

fn test_config() -> Config {
    Config {
        app_host: "127.0.0.1".to_string(),
        app_port: 3001,
        database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/miransas_test".to_string()
        }),
        database_max_connections: 1,
        admin_password: "test-admin-password".to_string(),
        jwt_secret: "test-jwt-secret-key-exactly-32chars".to_string(),
        secret_key: "test-secret-key-with-at-least-32c!".to_string(),
        cors_origin: "http://localhost:3000".to_string(),
        public_db_host: "panel.miransas.com".to_string(),
        public_db_port: 5432,
        public_db_name: "miransas".to_string(),
    }
}

fn test_state() -> AppState {
    let config = test_config();
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&config.database_url)
        .expect("test database URL should be valid");
    AppState::new(config, pool)
}

// ── Health ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn health_returns_200() {
    let app = build_router(test_state());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

// ── Auth ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn protected_route_without_token_is_401() {
    let app = build_router(test_state());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/projects")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_with_wrong_password_is_401() {
    let app = build_router(test_state());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"password":"wrong"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Login with the correct password, extract the JWT, then call /api/projects.
/// The projects call may succeed (200) or fail with a DB error (500) depending
/// on whether a test database is running, but it must NOT be 401 — which would
/// mean the token was rejected by the auth middleware.
#[tokio::test]
async fn login_and_access_projects_with_token() {
    let state = test_state();

    // Step 1: Login → expect 200 OK and a token in the body.
    let login_response = build_router(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"password":"test-admin-password"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(login_response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let token = json["token"]
        .as_str()
        .expect("response must have a token field");

    // Step 2: Call /api/projects with the token.
    // We only assert that auth passed (not 401); the DB may or may not be up.
    let projects_response = build_router(state)
        .oneshot(
            Request::builder()
                .uri("/api/projects")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(
        projects_response.status(),
        StatusCode::UNAUTHORIZED,
        "a valid JWT must be accepted by the auth middleware"
    );
}

/// PUT with a random UUID that does not exist in the database should return 404.
///
/// Requires a running PostgreSQL database (pointed to by DATABASE_URL).
/// Run with: cargo test -- --include-ignored
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn put_unknown_project_returns_404() {
    let state = test_state();

    // Get a token first.
    let login_response = build_router(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"password":"test-admin-password"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let body_bytes = axum::body::to_bytes(login_response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let token = json["token"].as_str().unwrap().to_string();

    // PUT with a UUID that will never exist.
    let fake_id = "00000000-0000-0000-0000-000000000000";
    let response = build_router(state)
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/projects/{fake_id}"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(r#"{"name":"updated"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
