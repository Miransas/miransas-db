use sqlx::PgPool;

use crate::{
    errors::AppError,
    models::{CreateDatabaseRequest, DatabaseMetadata},
    utils::crypto,
};

use super::shared::{empty_to_none, insert_audit_log, required_text, validate_port};

pub async fn list_databases(pool: &PgPool) -> Result<Vec<DatabaseMetadata>, AppError> {
    let rows = sqlx::query_as::<_, DatabaseMetadata>(
        r#"
        SELECT id, project_id, name, engine, host, port,
               database_name, username, notes, created_at, updated_at
        FROM   databases
        ORDER  BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn create_database(
    pool: &PgPool,
    secret_key: &str,
    input: CreateDatabaseRequest,
) -> Result<DatabaseMetadata, AppError> {
    let name = required_text("name", input.name)?;
    let engine = required_text("engine", input.engine)?;
    validate_port(input.port)?;

    // Encrypt the connection URL if provided.
    let connection_url_encrypted = input
        .connection_url
        .filter(|u| !u.trim().is_empty())
        .map(|u| crypto::encrypt(secret_key, &u))
        .transpose()?;

    let db = sqlx::query_as::<_, DatabaseMetadata>(
        r#"
        INSERT INTO databases
            (project_id, name, engine, host, port,
             database_name, username, notes, connection_url_encrypted)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, project_id, name, engine, host, port,
                  database_name, username, notes, created_at, updated_at
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
        Some(db.id),
        Some(format!("registered database {}", db.name)),
    )
    .await?;

    Ok(db)
}
