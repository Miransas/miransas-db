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
        database_url: std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/miransas_test".to_string()),
        database_max_connections: 1,
        admin_password: "test-admin-password".to_string(),
        jwt_secret: "test-jwt-secret-key-exactly-32chars".to_string(),
        secret_key: "test-secret-key-with-at-least-32c!".to_string(),
        cors_origin: "http://localhost:3000".to_string(),
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
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
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
