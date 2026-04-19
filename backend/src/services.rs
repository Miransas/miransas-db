use sqlx::{Connection, PgPool, Row};
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{
        AdminSummary, CreateDatabaseRequest, CreateProjectRequest, CreateSecretRequest,
        DatabaseMetadata, Project, QueryResult, SecretMetadata, TableDataResponse, TableInfo,
    },
    utils::{crypto, time},
};

// ── Projects ──────────────────────────────────────────────────────────────────

pub async fn list_projects(pool: &PgPool) -> Result<Vec<Project>, AppError> {
    let projects = sqlx::query_as::<_, Project>(
        r#"
        SELECT id, name, description, repository_url, created_at, updated_at
        FROM projects
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(projects)
}

pub async fn create_project(
    pool: &PgPool,
    input: CreateProjectRequest,
) -> Result<Project, AppError> {
    let name = required_text("name", input.name)?;

    let project = sqlx::query_as::<_, Project>(
        r#"
        INSERT INTO projects (name, description, repository_url)
        VALUES ($1, $2, $3)
        RETURNING id, name, description, repository_url, created_at, updated_at
        "#,
    )
    .bind(name)
    .bind(empty_to_none(input.description))
    .bind(empty_to_none(input.repository_url))
    .fetch_one(pool)
    .await?;

    insert_audit_log(
        pool,
        "create",
        "project",
        Some(project.id),
        Some(format!("created project {}", project.name)),
    )
    .await?;

    Ok(project)
}

// ── Databases ─────────────────────────────────────────────────────────────────

pub async fn list_databases(pool: &PgPool) -> Result<Vec<DatabaseMetadata>, AppError> {
    let databases = sqlx::query_as::<_, DatabaseMetadata>(
        r#"
        SELECT id, project_id, name, engine, host, port, database_name, username, notes, created_at, updated_at
        FROM databases
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(databases)
}

pub async fn create_database(
    pool: &PgPool,
    secret_key: &str,
    input: CreateDatabaseRequest,
) -> Result<DatabaseMetadata, AppError> {
    let name = required_text("name", input.name)?;
    let engine = required_text("engine", input.engine)?;
    validate_port(input.port)?;

    let connection_url_encrypted = input
        .connection_url
        .filter(|u| !u.trim().is_empty())
        .map(|u| crypto::encrypt(secret_key, &u))
        .transpose()?;

    let database = sqlx::query_as::<_, DatabaseMetadata>(
        r#"
        INSERT INTO databases (project_id, name, engine, host, port, database_name, username, notes, connection_url_encrypted)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, project_id, name, engine, host, port, database_name, username, notes, created_at, updated_at
        "#,
    )
    .bind(input.project_id)
    .bind(name)
    .bind(engine)
    .bind(empty_to_none(input.host))
    .bind(input.port)
    .bind(empty_to_none(input.database_name))
    .bind(empty_to_none(input.username))
    .bind(empty_to_none(input.notes))
    .bind(connection_url_encrypted)
    .fetch_one(pool)
    .await?;

    insert_audit_log(
        pool,
        "create",
        "database",
        Some(database.id),
        Some(format!("created database metadata {}", database.name)),
    )
    .await?;

    Ok(database)
}

// ── Table exploration ─────────────────────────────────────────────────────────

/// List all non-system tables in the external database registered under `db_id`.
pub async fn list_tables(
    pool: &PgPool,
    db_id: Uuid,
    secret_key: &str,
) -> Result<Vec<TableInfo>, AppError> {
    let url = get_connection_url(pool, db_id, secret_key).await?;
    let mut conn = sqlx::postgres::PgConnection::connect(&url).await?;

    let tables = sqlx::query_as::<_, TableInfo>(
        r#"
        SELECT
            table_schema AS schema,
            table_name   AS name,
            table_type
        FROM information_schema.tables
        WHERE table_schema NOT IN ('information_schema', 'pg_catalog', 'pg_toast')
        ORDER BY table_schema, table_name
        "#,
    )
    .fetch_all(&mut conn)
    .await?;

    Ok(tables)
}

/// Return paginated rows from a table in the external database.
/// `table` may be `"table_name"` or `"schema.table_name"`.
pub async fn get_table_data(
    pool: &PgPool,
    db_id: Uuid,
    table: &str,
    page: i64,
    page_size: i64,
    secret_key: &str,
) -> Result<TableDataResponse, AppError> {
    let quoted = validate_and_quote_table(table)?;
    let (schema_name, table_name) = parse_schema_table(table);

    let url = get_connection_url(pool, db_id, secret_key).await?;
    let mut conn = sqlx::postgres::PgConnection::connect(&url).await?;

    let page_size = page_size.clamp(1, 200);
    let page = page.max(1);
    let offset = (page - 1) * page_size;

    let total: i64 =
        sqlx::query_scalar(&format!("SELECT COUNT(*)::BIGINT FROM {quoted}"))
            .fetch_one(&mut conn)
            .await?;

    let data_sql = format!(
        "SELECT row_to_json(_t)::TEXT AS _row \
         FROM (SELECT * FROM {quoted} LIMIT {page_size} OFFSET {offset}) _t"
    );
    let rows_raw: Vec<String> = sqlx::query_scalar(&data_sql)
        .fetch_all(&mut conn)
        .await?;

    // Column names from information_schema (best-effort; empty vec on failure).
    let columns: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT column_name::TEXT
        FROM information_schema.columns
        WHERE table_schema = $1 AND table_name = $2
        ORDER BY ordinal_position
        "#,
    )
    .bind(schema_name)
    .bind(table_name)
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

// ── Raw SQL execution ─────────────────────────────────────────────────────────

/// Execute arbitrary SQL against the external database registered under `db_id`.
/// SELECT / WITH statements return rows as JSON; everything else returns rows_affected.
pub async fn execute_query(
    pool: &PgPool,
    db_id: Uuid,
    sql: &str,
    secret_key: &str,
) -> Result<QueryResult, AppError> {
    if sql.trim().is_empty() {
        return Err(AppError::BadRequest("sql must not be empty".to_string()));
    }

    let url = get_connection_url(pool, db_id, secret_key).await?;
    let mut conn = sqlx::postgres::PgConnection::connect(&url).await?;

    // Log the query (truncated) to our audit trail.
    let preview = &sql[..sql.len().min(500)];
    insert_audit_log(
        pool,
        "execute_query",
        "database",
        Some(db_id),
        Some(format!("SQL: {preview}")),
    )
    .await?;

    let first_word = sql
        .trim()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_uppercase();

    if matches!(first_word.as_str(), "SELECT" | "WITH" | "VALUES" | "TABLE") {
        // Wrap in row_to_json so we get structured JSON back.
        let wrapped = format!(
            "SELECT row_to_json(_q)::TEXT AS _row FROM ({}) _q LIMIT 10000",
            sql
        );
        let rows_raw: Vec<String> = sqlx::query_scalar(&wrapped)
            .fetch_all(&mut conn)
            .await?;

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

// ── Secrets ───────────────────────────────────────────────────────────────────

pub async fn list_secrets(pool: &PgPool) -> Result<Vec<SecretMetadata>, AppError> {
    let secrets = sqlx::query_as::<_, SecretMetadata>(
        r#"
        SELECT id, project_id, name, notes, created_at, updated_at
        FROM secrets
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(secrets)
}

pub async fn create_secret(
    pool: &PgPool,
    secret_key: &str,
    input: CreateSecretRequest,
) -> Result<SecretMetadata, AppError> {
    let name = required_text("name", input.name)?;
    let value = required_text("value", input.value)?;
    let encrypted_value = crypto::encrypt(secret_key, &value)?;

    let secret = sqlx::query_as::<_, SecretMetadata>(
        r#"
        INSERT INTO secrets (project_id, name, value_encrypted, notes)
        VALUES ($1, $2, $3, $4)
        RETURNING id, project_id, name, notes, created_at, updated_at
        "#,
    )
    .bind(input.project_id)
    .bind(name)
    .bind(encrypted_value)
    .bind(empty_to_none(input.notes))
    .fetch_one(pool)
    .await?;

    insert_audit_log(
        pool,
        "create",
        "secret",
        Some(secret.id),
        Some(format!("created secret metadata {}", secret.name)),
    )
    .await?;

    Ok(secret)
}

// ── Admin ─────────────────────────────────────────────────────────────────────

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

    Ok(AdminSummary {
        generated_at: time::now(),
        ..summary
    })
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Fetch and decrypt the stored connection URL for a registered database.
async fn get_connection_url(
    pool: &PgPool,
    id: Uuid,
    secret_key: &str,
) -> Result<String, AppError> {
    let row = sqlx::query("SELECT connection_url_encrypted FROM databases WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("database {id} not found")))?;

    let encrypted: Option<String> = row.try_get("connection_url_encrypted").unwrap_or(None);
    let encrypted = encrypted.ok_or_else(|| {
        AppError::BadRequest(
            "this database has no connection URL stored; provide one when registering".to_string(),
        )
    })?;

    crypto::decrypt(secret_key, &encrypted).map_err(AppError::Crypto)
}

/// Validate a `schema.table` or bare `table` identifier and return it
/// double-quoted for safe interpolation into SQL strings.
fn validate_and_quote_table(name: &str) -> Result<String, AppError> {
    let parts: Vec<&str> = name.splitn(2, '.').collect();
    for part in &parts {
        if part.is_empty() || !part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(AppError::BadRequest(format!(
                "invalid table identifier: {part:?}"
            )));
        }
    }
    Ok(parts
        .iter()
        .map(|p| format!("\"{p}\""))
        .collect::<Vec<_>>()
        .join("."))
}

/// Split `"schema.table"` → `("schema", "table")`, defaulting to `"public"`.
fn parse_schema_table(name: &str) -> (String, String) {
    match name.split_once('.') {
        Some((schema, table)) => (schema.to_string(), table.to_string()),
        None => ("public".to_string(), name.to_string()),
    }
}

async fn insert_audit_log(
    pool: &PgPool,
    action: &str,
    resource_type: &str,
    resource_id: Option<Uuid>,
    message: Option<String>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO audit_logs (action, resource_type, resource_id, message)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(message)
    .execute(pool)
    .await?;

    Ok(())
}

fn required_text(field: &str, value: String) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(format!("{field} is required")));
    }
    Ok(trimmed.to_string())
}

fn empty_to_none(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn validate_port(port: Option<i32>) -> Result<(), AppError> {
    if let Some(port) = port {
        if !(1..=65535).contains(&port) {
            return Err(AppError::BadRequest(
                "port must be between 1 and 65535".to_string(),
            ));
        }
    }
    Ok(())
}
