use sqlx::{Connection, PgPool, Row};
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{
        CreateProjectRequest, Project, QueryResult, TableDataResponse, TableInfo,
        UpdateProjectRequest,
    },
    utils::crypto,
};

use super::query::{rows_as_json, run_sql};
use super::shared::{
    empty_to_none, insert_audit_log, required_text, split_schema_table, validate_and_quote,
    validate_and_quote_col,
};

// ── CRUD ──────────────────────────────────────────────────────────────────────

pub async fn list_projects(pool: &PgPool) -> Result<Vec<Project>, AppError> {
    let projects = sqlx::query_as::<_, Project>(
        r#"
        SELECT id, name, description, repository_url, created_at, updated_at
        FROM   projects
        ORDER  BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(projects)
}

/// Create a project and, if `connection_string` is provided, encrypt and store it.
pub async fn create_project(
    pool: &PgPool,
    secret_key: &str,
    input: CreateProjectRequest,
) -> Result<Project, AppError> {
    let name = required_text("name", input.name)?;

    let conn_enc = input
        .connection_string
        .filter(|s| !s.trim().is_empty())
        .map(|s| crypto::encrypt(secret_key, s.trim()))
        .transpose()?;

    let project = sqlx::query_as::<_, Project>(
        r#"
        INSERT INTO projects (name, description, repository_url, connection_string_encrypted)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, description, repository_url, created_at, updated_at
        "#,
    )
    .bind(&name)
    .bind(empty_to_none(input.description))
    .bind(empty_to_none(input.repository_url))
    .bind(conn_enc)
    .fetch_one(pool)
    .await?;

    insert_audit_log(
        pool,
        "create",
        "project",
        Some(project.id),
        Some(format!("created project {name}")),
    )
    .await?;

    Ok(project)
}

// ── Project-database exploration ──────────────────────────────────────────────

/// List all user-visible tables in the project's database.
pub async fn list_project_tables(
    pool: &PgPool,
    project_id: Uuid,
    secret_key: &str,
) -> Result<Vec<TableInfo>, AppError> {
    let url = project_connection_url(pool, project_id, secret_key).await?;
    let mut conn = sqlx::postgres::PgConnection::connect(&url).await?;

    let tables = sqlx::query_as::<_, TableInfo>(
        r#"
        SELECT table_schema AS schema,
               table_name   AS name,
               table_type
        FROM   information_schema.tables
        WHERE  table_schema NOT IN ('information_schema', 'pg_catalog', 'pg_toast')
        ORDER  BY table_schema, table_name
        "#,
    )
    .fetch_all(&mut conn)
    .await?;

    Ok(tables)
}

/// Return paginated rows from a table in the project's database.
///
/// `table` accepts `"table_name"` (defaults to schema `public`) or
/// `"schema.table_name"`.
pub async fn get_project_table_data(
    pool: &PgPool,
    project_id: Uuid,
    table: &str,
    page: i64,
    page_size: i64,
    secret_key: &str,
) -> Result<TableDataResponse, AppError> {
    let quoted = validate_and_quote(table)?;
    let (schema_name, table_name) = split_schema_table(table);

    let url = project_connection_url(pool, project_id, secret_key).await?;
    let mut conn = sqlx::postgres::PgConnection::connect(&url).await?;

    let offset = (page - 1) * page_size;

    // Total row count.
    let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*)::BIGINT FROM {quoted}"))
        .fetch_one(&mut conn)
        .await?;

    // Rows serialised as JSON by Postgres.
    let rows_raw: Vec<String> = sqlx::query_scalar(&format!(
        "SELECT row_to_json(_t)::TEXT \
         FROM (SELECT * FROM {quoted} LIMIT {page_size} OFFSET {offset}) _t"
    ))
    .fetch_all(&mut conn)
    .await?;

    // Column names (ordered) from information_schema.
    let columns: Vec<String> = sqlx::query_scalar(
        "SELECT column_name::TEXT \
         FROM information_schema.columns \
         WHERE table_schema = $1 AND table_name = $2 \
         ORDER BY ordinal_position",
    )
    .bind(&schema_name)
    .bind(&table_name)
    .fetch_all(&mut conn)
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

/// Execute arbitrary SQL against the project's database and return results as JSON.
///
/// - SELECT / WITH / EXPLAIN → up to 10 000 rows as JSON
/// - Everything else → rows_affected count
///
/// The full query (truncated to 500 chars) is written to the audit log.
pub async fn execute_project_query(
    pool: &PgPool,
    project_id: Uuid,
    sql: &str,
    secret_key: &str,
) -> Result<QueryResult, AppError> {
    let sql = sql.trim();
    if sql.is_empty() {
        return Err(AppError::BadRequest("sql must not be empty".to_string()));
    }

    insert_audit_log(
        pool,
        "execute_query",
        "project",
        Some(project_id),
        Some(format!("SQL: {}", &sql[..sql.len().min(500)])),
    )
    .await?;

    let url = project_connection_url(pool, project_id, secret_key).await?;
    let mut conn = sqlx::postgres::PgConnection::connect(&url).await?;

    run_sql(&mut conn, sql).await
}

/// Delete a single row from a table in the project's database.
///
/// `pk_col` is the primary-key column name (default `"id"`).
/// `row_id` is the value cast to text for comparison.
/// Returns 404 when no row was deleted.
pub async fn delete_project_row(
    pool: &PgPool,
    project_id: Uuid,
    table: &str,
    pk_col: &str,
    row_id: &str,
    secret_key: &str,
) -> Result<u64, AppError> {
    let quoted_table = validate_and_quote(table)?;
    let quoted_pk = validate_and_quote_col(pk_col)?;

    let url = project_connection_url(pool, project_id, secret_key).await?;
    let mut conn = sqlx::postgres::PgConnection::connect(&url).await?;

    // Cast the PK column to TEXT so this works for UUID, INT, BIGINT, and TEXT PKs
    // without knowing the column type at compile time.
    let sql = format!("DELETE FROM {quoted_table} WHERE {quoted_pk}::TEXT = $1");
    let result = sqlx::query(&sql).bind(row_id).execute(&mut conn).await?;

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
        Some(format!(
            "deleted row from {table} where {pk_col} = {row_id}"
        )),
    )
    .await?;

    Ok(affected)
}

/// Update a project's mutable fields. Absent fields keep their existing value;
/// an empty string sets the field to NULL (except `name`, which must remain non-empty).
pub async fn update_project(
    pool: &PgPool,
    secret_key: &str,
    id: Uuid,
    input: UpdateProjectRequest,
) -> Result<Project, AppError> {
    // A private struct so we can read the encrypted column too.
    #[derive(sqlx::FromRow)]
    struct ProjectFull {
        name: String,
        description: Option<String>,
        repository_url: Option<String>,
        connection_string_encrypted: Option<String>,
    }

    let existing = sqlx::query_as::<_, ProjectFull>(
        r#"
        SELECT name, description, repository_url, connection_string_encrypted
        FROM   projects
        WHERE  id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("project {id} not found")))?;

    let name = match input.name {
        Some(n) => required_text("name", n)?,
        None => existing.name,
    };

    let description = if input.description.is_some() {
        empty_to_none(input.description)
    } else {
        existing.description
    };

    let repository_url = if input.repository_url.is_some() {
        empty_to_none(input.repository_url)
    } else {
        existing.repository_url
    };

    let connection_string_encrypted = match input.connection_string {
        None => existing.connection_string_encrypted,
        Some(ref s) if s.trim().is_empty() => None,
        Some(s) => Some(crypto::encrypt(secret_key, s.trim())?),
    };

    let project = sqlx::query_as::<_, Project>(
        r#"
        UPDATE projects
        SET    name = $2,
               description = $3,
               repository_url = $4,
               connection_string_encrypted = $5
        WHERE  id = $1
        RETURNING id, name, description, repository_url, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(&name)
    .bind(description)
    .bind(repository_url)
    .bind(connection_string_encrypted)
    .fetch_one(pool)
    .await?;

    insert_audit_log(
        pool,
        "update",
        "project",
        Some(id),
        Some(format!("updated project {name}")),
    )
    .await?;

    Ok(project)
}

/// Delete a project by id. Returns 404 if not found.
pub async fn delete_project(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("project {id} not found")));
    }

    insert_audit_log(
        pool,
        "delete",
        "project",
        Some(id),
        Some(format!("deleted project {id}")),
    )
    .await?;

    Ok(())
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Fetch and AES-decrypt the connection string stored for a project.
/// Returns 404 if the project does not exist.
/// Returns 400 if the project has no connection string stored.
async fn project_connection_url(
    pool: &PgPool,
    project_id: Uuid,
    secret_key: &str,
) -> Result<String, AppError> {
    let row = sqlx::query("SELECT connection_string_encrypted FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("project {project_id} not found")))?;

    let encrypted: Option<String> = row.try_get("connection_string_encrypted").unwrap_or(None);

    let encrypted = encrypted.ok_or_else(|| {
        AppError::BadRequest(
            "project has no connection string stored; \
             provide `connection_string` when creating the project"
                .to_string(),
        )
    })?;

    crypto::decrypt(secret_key, &encrypted).map_err(AppError::Crypto)
}
