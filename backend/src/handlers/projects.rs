use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{
        CreateProjectRequest, DeleteRowQuery, PaginationQuery, Project, QueryRequest, QueryResult,
        TableDataResponse, TableInfo,
    },
    services,
    state::AppState,
};

// ── CRUD ──────────────────────────────────────────────────────────────────────

/// GET /api/projects
pub async fn list_projects(
    State(state): State<AppState>,
) -> Result<Json<Vec<Project>>, AppError> {
    Ok(Json(services::list_projects(&state.pool).await?))
}

/// POST /api/projects
/// Body: `{ "name": "...", "connection_string": "postgres://..." }`
pub async fn create_project(
    State(state): State<AppState>,
    Json(input): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<Project>), AppError> {
    Ok((
        StatusCode::CREATED,
        Json(
            services::create_project(&state.pool, &state.config.secret_key, input).await?,
        ),
    ))
}

// ── Project-database exploration ──────────────────────────────────────────────

/// GET /api/projects/:id/tables
///
/// Lists all user-visible tables in the project's database.
pub async fn list_tables(
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<TableInfo>>, AppError> {
    Ok(Json(
        services::list_project_tables(&state.pool, project_id, &state.config.secret_key).await?,
    ))
}

/// GET /api/projects/:id/tables/:table?page=1&limit=50
///
/// Returns paginated rows. `:table` can be `"table_name"` or `"schema.table_name"`.
/// Query params: `page` (default 1), `limit` or `page_size` (default 50, max 200).
pub async fn get_table_data(
    State(state): State<AppState>,
    Path((project_id, table)): Path<(Uuid, String)>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<TableDataResponse>, AppError> {
    let page = pagination.resolved_page();
    let limit = pagination.resolved_limit();
    Ok(Json(
        services::get_project_table_data(
            &state.pool,
            project_id,
            &table,
            page,
            limit,
            &state.config.secret_key,
        )
        .await?,
    ))
}

/// POST /api/projects/:id/query
/// Body: `{ "sql": "SELECT ..." }`
///
/// Executes arbitrary SQL against the project's database.
/// SELECT/WITH → rows as JSON (max 10 000).
/// INSERT/UPDATE/DELETE/DDL → rows_affected count.
pub async fn execute_query(
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Json(input): Json<QueryRequest>,
) -> Result<Json<QueryResult>, AppError> {
    Ok(Json(
        services::execute_project_query(
            &state.pool,
            project_id,
            &input.sql,
            &state.config.secret_key,
        )
        .await?,
    ))
}

/// DELETE /api/projects/:id/tables/:table/:row_id?pk=id
///
/// Deletes the row where `pk` column (default `"id"`) equals `:row_id`.
/// The PK value is compared as text so it works for UUID, INT, and TEXT PKs.
pub async fn delete_row(
    State(state): State<AppState>,
    Path((project_id, table, row_id)): Path<(Uuid, String, String)>,
    Query(params): Query<DeleteRowQuery>,
) -> Result<StatusCode, AppError> {
    let pk_col = params.pk.as_deref().unwrap_or("id");
    services::delete_project_row(
        &state.pool,
        project_id,
        &table,
        pk_col,
        &row_id,
        &state.config.secret_key,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
