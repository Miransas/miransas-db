use axum::{
    extract::{Path, Query, State},
    Json,
};
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{QueryHistoryFilter, QueryHistoryResponse},
    services,
    state::AppState,
};

/// GET /api/projects/:project_id/query-history
pub async fn list_history(
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Query(filter): Query<QueryHistoryFilter>,
) -> Result<Json<QueryHistoryResponse>, AppError> {
    Ok(Json(
        services::query_log::list(&state.pool, project_id, filter).await?,
    ))
}
