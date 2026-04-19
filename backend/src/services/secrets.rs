use sqlx::PgPool;

use crate::{
    errors::AppError,
    models::{CreateSecretRequest, SecretMetadata},
    utils::crypto,
};

use super::shared::{empty_to_none, insert_audit_log, required_text};

pub async fn list_secrets(pool: &PgPool) -> Result<Vec<SecretMetadata>, AppError> {
    let rows = sqlx::query_as::<_, SecretMetadata>(
        r#"
        SELECT id, project_id, name, notes, created_at, updated_at
        FROM   secrets
        ORDER  BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
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
        Some(format!("stored secret {}", secret.name)),
    )
    .await?;

    Ok(secret)
}
