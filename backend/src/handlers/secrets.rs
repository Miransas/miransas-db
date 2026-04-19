use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{CreateSecretRequest, SecretMetadata, SecretWithValue},
    services,
    state::AppState,
};

/// GET /api/secrets/:id/reveal
///
/// Decrypts and returns the secret value. ALWAYS writes an audit log so every
/// plaintext read is traceable (action = "reveal").
pub async fn reveal_secret(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SecretWithValue>, AppError> {
    Ok(Json(
        services::reveal_secret(&state.pool, &state.config.secret_key, id).await?,
    ))
}

/// DELETE /api/secrets/:id — returns 204, 404 if not found
pub async fn delete_secret(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    services::delete_secret(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_secrets(
    State(state): State<AppState>,
) -> Result<Json<Vec<SecretMetadata>>, AppError> {
    Ok(Json(services::list_secrets(&state.pool).await?))
}

pub async fn create_secret(
    State(state): State<AppState>,
    Json(input): Json<CreateSecretRequest>,
) -> Result<(StatusCode, Json<SecretMetadata>), AppError> {
    Ok((
        StatusCode::CREATED,
        Json(services::create_secret(&state.pool, &state.config.secret_key, input).await?),
    ))
}
