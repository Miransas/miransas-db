use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{QueryHistoryEntry, QueryHistoryFilter, QueryHistoryResponse},
};

pub async fn list(
    pool: &PgPool,
    project_id: Uuid,
    filter: QueryHistoryFilter,
) -> Result<QueryHistoryResponse, AppError> {
    let page = filter.page.unwrap_or(1).max(1);
    let limit = filter.limit.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * limit;

    let total: i64 = if let Some(s) = filter.success {
        sqlx::query_scalar(
            "SELECT COUNT(*)::BIGINT FROM query_history \
             WHERE project_id = $1 AND success = $2",
        )
        .bind(project_id)
        .bind(s)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(*)::BIGINT FROM query_history WHERE project_id = $1",
        )
        .bind(project_id)
        .fetch_one(pool)
        .await?
    };

    let rows: Vec<QueryHistoryEntry> = if let Some(s) = filter.success {
        sqlx::query_as::<_, QueryHistoryEntry>(
            "SELECT id, project_id, sql, duration_ms, rows_affected, success, \
                    error_message, executed_at \
             FROM query_history \
             WHERE project_id = $1 AND success = $2 \
             ORDER BY executed_at DESC \
             LIMIT $3 OFFSET $4",
        )
        .bind(project_id)
        .bind(s)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, QueryHistoryEntry>(
            "SELECT id, project_id, sql, duration_ms, rows_affected, success, \
                    error_message, executed_at \
             FROM query_history \
             WHERE project_id = $1 \
             ORDER BY executed_at DESC \
             LIMIT $2 OFFSET $3",
        )
        .bind(project_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    };

    Ok(QueryHistoryResponse {
        rows,
        total,
        page,
        page_size: limit,
    })
}
