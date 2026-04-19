//! External-database operations: table listing, row pagination, raw SQL execution.
//!
//! Every function opens a *direct* connection to the external database whose
//! connection URL is stored encrypted in our internal `databases` table.

use sqlx::{Connection, PgPool, Row};
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{QueryResult, TableDataResponse, TableInfo},
    utils::crypto,
};

use super::shared::insert_audit_log;

// ── Public surface ────────────────────────────────────────────────────────────

/// List all user-visible tables in the external database.
pub async fn list_tables(
    pool: &PgPool,
    db_id: Uuid,
    secret_key: &str,
) -> Result<Vec<TableInfo>, AppError> {
    let url = connection_url(pool, db_id, secret_key).await?;
    let mut conn = sqlx::postgres::PgConnection::connect(&url).await?;

    let tables = sqlx::query_as::<_, TableInfo>(
        r#"
        SELECT
            table_schema AS schema,
            table_name   AS name,
            table_type
        FROM  information_schema.tables
        WHERE table_schema NOT IN ('information_schema', 'pg_catalog', 'pg_toast')
        ORDER BY table_schema, table_name
        "#,
    )
    .fetch_all(&mut conn)
    .await?;

    Ok(tables)
}

/// Return a paginated view of an external table's rows.
///
/// `table` accepts `"table_name"` (defaults to schema `public`) or
/// `"schema.table_name"`.
pub async fn get_table_data(
    pool: &PgPool,
    db_id: Uuid,
    table: &str,
    page: i64,
    page_size: i64,
    secret_key: &str,
) -> Result<TableDataResponse, AppError> {
    let quoted = validate_and_quote(table)?;
    let (schema_name, table_name) = split_schema_table(table);

    let url = connection_url(pool, db_id, secret_key).await?;
    let mut conn = sqlx::postgres::PgConnection::connect(&url).await?;

    let page_size = page_size.clamp(1, 200);
    let page = page.max(1);
    let offset = (page - 1) * page_size;

    // Total row count.
    let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*)::BIGINT FROM {quoted}"))
        .fetch_one(&mut conn)
        .await?;

    // Rows as JSON text.
    let rows_raw: Vec<String> = sqlx::query_scalar(&format!(
        "SELECT row_to_json(_t)::TEXT \
         FROM (SELECT * FROM {quoted} LIMIT {page_size} OFFSET {offset}) _t"
    ))
    .fetch_all(&mut conn)
    .await?;

    // Column names from information_schema (best-effort).
    let columns: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT column_name::TEXT
        FROM   information_schema.columns
        WHERE  table_schema = $1 AND table_name = $2
        ORDER  BY ordinal_position
        "#,
    )
    .bind(&schema_name)
    .bind(&table_name)
    .fetch_all(&mut conn)
    .await
    .unwrap_or_default();

    let rows: Vec<serde_json::Value> = rows_raw
        .iter()
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect();

    Ok(TableDataResponse {
        columns,
        rows,
        total,
        page,
        page_size,
    })
}

/// Execute arbitrary SQL against the external database.
///
/// - `SELECT` / `WITH` / `VALUES` → returns rows as JSON (max 10 000)
/// - Everything else → returns `rows_affected`
///
/// The query is also written to the audit log (truncated to 500 chars).
pub async fn execute_query(
    pool: &PgPool,
    db_id: Uuid,
    sql: &str,
    secret_key: &str,
) -> Result<QueryResult, AppError> {
    let sql = sql.trim();
    if sql.is_empty() {
        return Err(AppError::BadRequest("sql must not be empty".to_string()));
    }

    // Audit log first (uses internal pool).
    insert_audit_log(
        pool,
        "execute_query",
        "database",
        Some(db_id),
        Some(format!("SQL: {}", &sql[..sql.len().min(500)])),
    )
    .await?;

    let url = connection_url(pool, db_id, secret_key).await?;
    let mut conn = sqlx::postgres::PgConnection::connect(&url).await?;

    let first_word = sql.split_whitespace().next().unwrap_or("").to_uppercase();

    if matches!(
        first_word.as_str(),
        "SELECT" | "WITH" | "VALUES" | "TABLE" | "EXPLAIN"
    ) {
        let wrapped = format!("SELECT row_to_json(_q)::TEXT FROM ({sql}) _q LIMIT 10000");
        let rows_raw: Vec<String> = sqlx::query_scalar(&wrapped).fetch_all(&mut conn).await?;

        let rows: Vec<serde_json::Value> = rows_raw
            .iter()
            .filter_map(|s| serde_json::from_str(s).ok())
            .collect();

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
        let result = sqlx::query(sql).execute(&mut conn).await?;
        let affected = result.rows_affected();
        Ok(QueryResult {
            columns: vec![],
            rows: vec![],
            rows_affected: Some(affected),
            message: format!("{affected} rows affected"),
        })
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Fetch and decrypt the connection URL stored for the registered database.
async fn connection_url(pool: &PgPool, id: Uuid, secret_key: &str) -> Result<String, AppError> {
    let row = sqlx::query("SELECT connection_url_encrypted FROM databases WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("database {id} not found")))?;

    let encrypted: Option<String> = row.try_get("connection_url_encrypted").unwrap_or(None);
    let encrypted = encrypted.ok_or_else(|| {
        AppError::BadRequest(
            "database has no connection URL; supply `connection_url` when registering".to_string(),
        )
    })?;

    crypto::decrypt(secret_key, &encrypted).map_err(AppError::Crypto)
}

/// Validate a `[schema.]table` identifier and return it SQL-safe (double-quoted).
/// Only ASCII alphanumerics and underscores are allowed in each part.
fn validate_and_quote(name: &str) -> Result<String, AppError> {
    let parts: Vec<&str> = name.splitn(2, '.').collect();
    for part in &parts {
        if part.is_empty() || !part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(AppError::BadRequest(format!(
                "invalid identifier: {part:?} (only [a-zA-Z0-9_] allowed)"
            )));
        }
    }
    Ok(parts
        .iter()
        .map(|p| format!("\"{p}\""))
        .collect::<Vec<_>>()
        .join("."))
}

/// Split `"schema.table"` → `("schema", "table")`, defaulting schema to `"public"`.
fn split_schema_table(name: &str) -> (String, String) {
    match name.split_once('.') {
        Some((s, t)) => (s.to_string(), t.to_string()),
        None => ("public".to_string(), name.to_string()),
    }
}
