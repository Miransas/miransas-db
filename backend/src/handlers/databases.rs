use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{
        CreateDatabaseRequest, DatabaseMetadata, PaginationQuery, QueryRequest, QueryResult,
        TableDataResponse, TableInfo,
    },
    services,
    state::AppState,
};

pub async fn list_databases(
    State(state): State<AppState>,
) -> Result<Json<Vec<DatabaseMetadata>>, AppError> {
    Ok(Json(services::list_databases(&state.pool).await?))
}

pub async fn create_database(
    State(state): State<AppState>,
    Json(input): Json<CreateDatabaseRequest>,
) -> Result<(StatusCode, Json<DatabaseMetadata>), AppError> {
    Ok((
        StatusCode::CREATED,
        Json(services::create_database(&state.pool, &state.config.secret_key, input).await?),
    ))
}

pub async fn list_tables(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<TableInfo>>, AppError> {
    Ok(Json(
        services::list_tables(&state.pool, id, &state.config.secret_key).await?,
    ))
}

pub async fn get_table_data(
    State(state): State<AppState>,
    Path((id, table)): Path<(Uuid, String)>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<TableDataResponse>, AppError> {
    Ok(Json(
        services::get_table_data(
            &state.pool,
            id,
            &table,
            pagination.resolved_page(),
            pagination.resolved_limit(),
            &state.config.secret_key,
        )
        .await?,
    ))
}

pub async fn execute_query(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<QueryRequest>,
) -> Result<Json<QueryResult>, AppError> {
    Ok(Json(
        services::execute_query(&state.pool, id, &input.sql, &state.config.secret_key).await?,
    ))
}
