use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{
        ConnectionInfo, CreateProjectRequest, DeleteRowQuery, PaginationQuery, Project,
        ProjectResetPasswordResponse, QueryRequest, QueryResult, TableDataResponse, TableInfo,
        UpdateProjectRequest,
    },
    services,
    state::AppState,
};

// ── CRUD ──────────────────────────────────────────────────────────────────────

/// GET /api/projects
pub async fn list_projects(State(state): State<AppState>) -> Result<Json<Vec<Project>>, AppError> {
    Ok(Json(services::list_projects(&state.pool).await?))
}

/// GET /api/projects/:id
pub async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Project>, AppError> {
    Ok(Json(services::get_project(&state.pool, id).await?))
}

/// POST /api/projects
pub async fn create_project(
    State(state): State<AppState>,
    Json(input): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<Project>), AppError> {
    Ok((
        StatusCode::CREATED,
        Json(services::create_project(&state.pool, &state.config.secret_key, input).await?),
    ))
}

/// PUT /api/projects/:id
pub async fn update_project(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateProjectRequest>,
) -> Result<Json<Project>, AppError> {
    Ok(Json(services::update_project(&state.pool, id, input).await?))
}

/// DELETE /api/projects/:id
pub async fn delete_project(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    services::delete_project(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Project schema exploration ────────────────────────────────────────────────

/// GET /api/projects/:id/tables
pub async fn list_tables(
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<TableInfo>>, AppError> {
    Ok(Json(
        services::list_project_tables(&state.pool, project_id).await?,
    ))
}

/// GET /api/projects/:id/tables/:table?page=1&limit=50
pub async fn get_table_data(
    State(state): State<AppState>,
    Path((project_id, table)): Path<(Uuid, String)>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<TableDataResponse>, AppError> {
    let page = pagination.resolved_page();
    let limit = pagination.resolved_limit();
    Ok(Json(
        services::get_project_table_data(&state.pool, project_id, &table, page, limit).await?,
    ))
}

/// POST /api/projects/:id/query
pub async fn execute_query(
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Json(input): Json<QueryRequest>,
) -> Result<Json<QueryResult>, AppError> {
    Ok(Json(
        services::execute_project_query(&state.pool, project_id, &input.sql).await?,
    ))
}

/// GET /api/projects/:id/connection
pub async fn get_connection(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ConnectionInfo>, AppError> {
    Ok(Json(
        services::get_connection_info(&state.pool, &state.config, id).await?,
    ))
}

/// POST /api/projects/:id/reset-password
pub async fn reset_project_password(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ProjectResetPasswordResponse>, AppError> {
    Ok(Json(
        services::reset_project_password(&state.pool, &state.config, id).await?,
    ))
}

/// DELETE /api/projects/:id/tables/:table/:row_id?pk=id
pub async fn delete_row(
    State(state): State<AppState>,
    Path((project_id, table, row_id)): Path<(Uuid, String, String)>,
    Query(params): Query<DeleteRowQuery>,
) -> Result<StatusCode, AppError> {
    let pk_col = params.pk.as_deref().unwrap_or("id");
    services::delete_project_row(&state.pool, project_id, &table, pk_col, &row_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
