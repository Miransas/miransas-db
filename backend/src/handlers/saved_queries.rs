use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{CreateSavedQueryRequest, SavedQuery, UpdateSavedQueryRequest},
    services,
    state::AppState,
};

/// GET /api/projects/:project_id/saved-queries
pub async fn list_saved_queries(
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<SavedQuery>>, AppError> {
    Ok(Json(
        services::saved_queries::list(&state.pool, project_id).await?,
    ))
}

/// GET /api/saved-queries/:id
pub async fn get_saved_query(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SavedQuery>, AppError> {
    Ok(Json(services::saved_queries::get(&state.pool, id).await?))
}

/// POST /api/projects/:project_id/saved-queries
pub async fn create_saved_query(
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Json(input): Json<CreateSavedQueryRequest>,
) -> Result<(StatusCode, Json<SavedQuery>), AppError> {
    Ok((
        StatusCode::CREATED,
        Json(services::saved_queries::create(&state.pool, project_id, input).await?),
    ))
}

/// PUT /api/saved-queries/:id
pub async fn update_saved_query(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateSavedQueryRequest>,
) -> Result<Json<SavedQuery>, AppError> {
    Ok(Json(
        services::saved_queries::update(&state.pool, id, input).await?,
    ))
}

/// DELETE /api/saved-queries/:id
pub async fn delete_saved_query(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    services::saved_queries::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
