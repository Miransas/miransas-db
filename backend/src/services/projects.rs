use sqlx::{PgPool, Row};
use std::time::Instant;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{
        CreateProjectRequest, Project, QueryResult, TableDataResponse, TableInfo,
        UpdateProjectRequest,
    },
};

use super::shared::{
    empty_to_none, ensure_safe_ident, generate_schema_name, get_schema_name, insert_audit_log,
    required_text,
};

// ── SQL helpers ───────────────────────────────────────────────────────────────

fn rows_as_json(raw: Vec<String>) -> Vec<serde_json::Value> {
    raw.iter()
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect()
}

async fn run_sql(
    conn: &mut sqlx::postgres::PgConnection,
    sql: &str,
) -> Result<QueryResult, AppError> {
    let first_word = sql.split_whitespace().next().unwrap_or("").to_uppercase();

    if matches!(
        first_word.as_str(),
        "SELECT" | "WITH" | "VALUES" | "TABLE" | "EXPLAIN"
    ) {
        let wrapped = format!("SELECT row_to_json(_q)::TEXT FROM ({sql}) _q LIMIT 10000");
        let rows_raw: Vec<String> = sqlx::query_scalar(&wrapped).fetch_all(&mut *conn).await?;
        let rows = rows_as_json(rows_raw);
        let columns: Vec<String> = rows
            .first()
            .and_then(|r| r.as_object())
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default();
        let count = rows.len();
        Ok(QueryResult {
            columns,
            rows,
            rows_affected: None,
            message: format!("{count} rows returned"),
        })
    } else {
        let result = sqlx::query(sql).execute(&mut *conn).await?;
        let affected = result.rows_affected();
        Ok(QueryResult {
            columns: vec![],
            rows: vec![],
            rows_affected: Some(affected),
            message: format!("{affected} rows affected"),
        })
    }
}

async fn insert_audit_log_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    action: &str,
    resource_type: &str,
    resource_id: Option<Uuid>,
    message: Option<String>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_logs (action, resource_type, resource_id, message) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(message)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

// ── CRUD ──────────────────────────────────────────────────────────────────────

pub async fn list_projects(pool: &PgPool) -> Result<Vec<Project>, AppError> {
    let projects = sqlx::query_as::<_, Project>(
        "SELECT id, name, description, repository_url, schema_name, created_at, updated_at \
         FROM projects ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(projects)
}

pub async fn get_project(pool: &PgPool, id: Uuid) -> Result<Project, AppError> {
    sqlx::query_as::<_, Project>(
        "SELECT id, name, description, repository_url, schema_name, created_at, updated_at \
         FROM projects WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("project {id} not found")))
}

pub async fn create_project(
    pool: &PgPool,
    input: CreateProjectRequest,
) -> Result<Project, AppError> {
    let name = required_text("name", input.name)?;
    let schema_name = generate_schema_name(pool, &name).await?;
    ensure_safe_ident(&schema_name)?;

    let mut tx = pool.begin().await?;

    let project = sqlx::query_as::<_, Project>(
        "INSERT INTO projects (name, description, repository_url, schema_name) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id, name, description, repository_url, schema_name, created_at, updated_at",
    )
    .bind(&name)
    .bind(empty_to_none(input.description))
    .bind(empty_to_none(input.repository_url))
    .bind(&schema_name)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(&format!("CREATE SCHEMA \"{}\"", schema_name))
        .execute(&mut *tx)
        .await?;

    insert_audit_log_tx(
        &mut tx,
        "create",
        "project",
        Some(project.id),
        Some(format!("created project {}", project.name)),
    )
    .await?;

    tx.commit().await?;
    Ok(project)
}

pub async fn update_project(
    pool: &PgPool,
    id: Uuid,
    input: UpdateProjectRequest,
) -> Result<Project, AppError> {
    if let Some(ref n) = input.name {
        required_text("name", n.clone())?;
    }

    let name_present = input.name.is_some();
    let name_val = input.name.clone().unwrap_or_default();
    let desc_present = input.description.is_some();
    let desc_val = input.description.clone().unwrap_or_default();
    let repo_present = input.repository_url.is_some();
    let repo_val = input.repository_url.clone().unwrap_or_default();

    let project = sqlx::query_as::<_, Project>(
        r#"
        UPDATE projects
        SET    name            = CASE WHEN $2 THEN $3            ELSE name            END,
               description    = CASE WHEN $4 THEN NULLIF($5, '') ELSE description    END,
               repository_url = CASE WHEN $6 THEN NULLIF($7, '') ELSE repository_url END,
               updated_at     = NOW()
        WHERE  id = $1
        RETURNING id, name, description, repository_url, schema_name, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(name_present)
    .bind(name_val)
    .bind(desc_present)
    .bind(desc_val)
    .bind(repo_present)
    .bind(repo_val)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("project {id} not found")))?;

    insert_audit_log(
        pool,
        "update",
        "project",
        Some(id),
        Some(format!("updated project {}", project.name)),
    )
    .await?;

    Ok(project)
}

pub async fn delete_project(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    let row = sqlx::query("SELECT schema_name FROM projects WHERE id = $1")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("project {id} not found")))?;

    let schema_name: String = row.try_get("schema_name")?;

    sqlx::query(&format!("DROP SCHEMA \"{}\" CASCADE", schema_name))
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    insert_audit_log_tx(
        &mut tx,
        "delete",
        "project",
        Some(id),
        Some(format!("deleted project {id}")),
    )
    .await?;

    tx.commit().await?;
    Ok(())
}

// ── Table exploration ─────────────────────────────────────────────────────────

pub async fn list_project_tables(
    pool: &PgPool,
    project_id: Uuid,
) -> Result<Vec<TableInfo>, AppError> {
    let schema_name = get_schema_name(pool, project_id).await?;

    let tables = sqlx::query_as::<_, TableInfo>(
        "SELECT table_schema AS schema, table_name AS name, table_type \
         FROM information_schema.tables \
         WHERE table_schema = $1 \
         ORDER BY table_name",
    )
    .bind(&schema_name)
    .fetch_all(pool)
    .await?;

    Ok(tables)
}

pub async fn get_project_table_data(
    pool: &PgPool,
    project_id: Uuid,
    table: &str,
    page: i64,
    page_size: i64,
) -> Result<TableDataResponse, AppError> {
    let schema_name = get_schema_name(pool, project_id).await?;
    ensure_safe_ident(table)?;
    let quoted = format!("\"{}\".\"{}\"", schema_name, table);

    let offset = (page - 1) * page_size;

    let total: i64 =
        sqlx::query_scalar(&format!("SELECT COUNT(*)::BIGINT FROM {quoted}"))
            .fetch_one(pool)
            .await?;

    let rows_raw: Vec<String> = sqlx::query_scalar(&format!(
        "SELECT row_to_json(_t)::TEXT \
         FROM (SELECT * FROM {quoted} LIMIT {page_size} OFFSET {offset}) _t"
    ))
    .fetch_all(pool)
    .await?;

    let columns: Vec<String> = sqlx::query_scalar(
        "SELECT column_name::TEXT \
         FROM information_schema.columns \
         WHERE table_schema = $1 AND table_name = $2 \
         ORDER BY ordinal_position",
    )
    .bind(&schema_name)
    .bind(table)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    Ok(TableDataResponse {
        columns,
        rows: rows_as_json(rows_raw),
        total,
        page,
        page_size,
    })
}

pub async fn execute_project_query(
    pool: &PgPool,
    project_id: Uuid,
    sql: &str,
) -> Result<QueryResult, AppError> {
    let sql = sql.trim();
    if sql.is_empty() {
        return Err(AppError::BadRequest("sql must not be empty".to_string()));
    }

    let schema_name = get_schema_name(pool, project_id).await?;

    let start = Instant::now();
    let mut tx = pool.begin().await?;

    sqlx::query(&format!(
        "SET LOCAL search_path TO \"{}\", public",
        schema_name
    ))
    .execute(&mut *tx)
    .await?;

    let result = run_sql(&mut *tx, sql).await;
    let duration_ms = start.elapsed().as_millis() as i32;

    // Commit or rollback the user's query — the search_path change is
    // scoped to this transaction and never leaks back to the pool.
    match &result {
        Ok(_) => { let _ = tx.commit().await; }
        Err(_) => { let _ = tx.rollback().await; }
    }

    let (success, err_msg, query_result) = match result {
        Ok(qr) => (true, None, Ok(qr)),
        Err(e) => {
            let msg = e.to_string();
            (false, Some(msg.clone()), Err(e))
        }
    };

    let rows_affected: Option<i64> = query_result
        .as_ref()
        .ok()
        .and_then(|qr| qr.rows_affected.map(|n| n as i64));

    let truncated: String = sql.chars().take(10000).collect();

    let _ = sqlx::query(
        "INSERT INTO query_history \
         (project_id, sql, duration_ms, rows_affected, success, error_message) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(project_id)
    .bind(truncated)
    .bind(duration_ms)
    .bind(rows_affected)
    .bind(success)
    .bind(err_msg)
    .execute(pool)
    .await;

    query_result
}

pub async fn delete_project_row(
    pool: &PgPool,
    project_id: Uuid,
    table: &str,
    pk_col: &str,
    row_id: &str,
) -> Result<u64, AppError> {
    let schema_name = get_schema_name(pool, project_id).await?;
    ensure_safe_ident(table)?;
    ensure_safe_ident(pk_col)?;

    let sql = format!(
        "DELETE FROM \"{}\".\"{}\" WHERE \"{}\"::TEXT = $1",
        schema_name, table, pk_col
    );
    let result = sqlx::query(&sql).bind(row_id).execute(pool).await?;

    let affected = result.rows_affected();
    if affected == 0 {
        return Err(AppError::NotFound(format!(
            "no row found in {table} where {pk_col} = {row_id}"
        )));
    }

    insert_audit_log(
        pool,
        "delete_row",
        "project",
        Some(project_id),
        Some(format!("deleted row from {table} where {pk_col} = {row_id}")),
    )
    .await?;

    Ok(affected)
}
