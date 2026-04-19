//! Private helpers shared across all service modules.

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;

pub async fn insert_audit_log(
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

/// Return `Err(BadRequest)` when a required text field is blank.
pub fn required_text(field: &str, value: String) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(format!("{field} is required")));
    }
    Ok(trimmed.to_string())
}

/// Convert an `Option<String>` that is blank / whitespace-only into `None`.
pub fn empty_to_none(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

/// Validate that an optional port number is in the TCP range 1–65535.
pub fn validate_port(port: Option<i32>) -> Result<(), AppError> {
    if let Some(p) = port {
        if !(1..=65535).contains(&p) {
            return Err(AppError::BadRequest(
                "port must be between 1 and 65535".to_string(),
            ));
        }
    }
    Ok(())
}
