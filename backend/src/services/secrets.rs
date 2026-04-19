use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{CreateSecretRequest, SecretMetadata, SecretWithValue},
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

/// Decrypt and return the secret value. ALWAYS writes an audit log so every
/// plaintext read is traceable.
pub async fn reveal_secret(
    pool: &PgPool,
    secret_key: &str,
    id: Uuid,
) -> Result<SecretWithValue, AppError> {
    let row = sqlx::query(
        r#"
        SELECT id, project_id, name, value_encrypted, notes
        FROM   secrets
        WHERE  id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("secret {id} not found")))?;

    let value_encrypted: String = row.try_get("value_encrypted").unwrap();
    let value = crypto::decrypt(secret_key, &value_encrypted)?;

    // Audit log BEFORE returning — every plaintext read must leave a trace.
    insert_audit_log(
        pool,
        "reveal",
        "secret",
        Some(id),
        Some(format!(
            "revealed secret {}",
            row.try_get::<String, _>("name").unwrap_or_default()
        )),
    )
    .await?;

    Ok(SecretWithValue {
        id: row.try_get("id").unwrap(),
        name: row.try_get("name").unwrap(),
        value,
        notes: row.try_get("notes").unwrap_or(None),
        project_id: row.try_get("project_id").unwrap_or(None),
    })
}

/// Delete a secret by id. Returns 404 if not found.
pub async fn delete_secret(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM secrets WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("secret {id} not found")));
    }

    insert_audit_log(
        pool,
        "delete",
        "secret",
        Some(id),
        Some(format!("deleted secret {id}")),
    )
    .await?;

    Ok(())
}
