use sqlx::PgPool;

use crate::{errors::AppError, models::AdminSummary, utils::time};

pub async fn admin_summary(pool: &PgPool) -> Result<AdminSummary, AppError> {
    let summary = sqlx::query_as::<_, AdminSummary>(
        r#"
        SELECT
            (SELECT COUNT(*) FROM projects)::BIGINT   AS project_count,
            (SELECT COUNT(*) FROM databases)::BIGINT  AS database_count,
            (SELECT COUNT(*) FROM secrets)::BIGINT    AS secret_count,
            (SELECT COUNT(*) FROM audit_logs)::BIGINT AS audit_log_count,
            NOW() AS generated_at
        "#,
    )
    .fetch_one(pool)
    .await?;

    // Override the DB timestamp with a precise Rust-side one.
    Ok(AdminSummary {
        generated_at: time::now(),
        ..summary
    })
}
