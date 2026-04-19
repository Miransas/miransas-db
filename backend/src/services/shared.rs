//! Private helpers shared across all service modules.

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;

// ── Audit log ─────────────────────────────────────────────────────────────────

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

// ── Input validation ──────────────────────────────────────────────────────────

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

// ── SQL identifier safety ─────────────────────────────────────────────────────

/// Validate a `[schema.]table` identifier and return it SQL-safe (double-quoted).
///
/// Each dot-separated part must contain only ASCII alphanumerics and underscores.
/// Returns `"schema"."table"` or `"table"`.
pub fn validate_and_quote(name: &str) -> Result<String, AppError> {
    let parts: Vec<&str> = name.splitn(2, '.').collect();
    for part in &parts {
        if part.is_empty() || !part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(AppError::BadRequest(format!(
                "invalid identifier: {part:?} — only [a-zA-Z0-9_] and a single dot separator are allowed"
            )));
        }
    }
    Ok(parts
        .iter()
        .map(|p| format!("\"{p}\""))
        .collect::<Vec<_>>()
        .join("."))
}

/// Validate a *single* identifier (no dot separator) and return it double-quoted.
/// Used for column names.
pub fn validate_and_quote_col(name: &str) -> Result<String, AppError> {
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(AppError::BadRequest(format!(
            "invalid column name: {name:?} — only [a-zA-Z0-9_] allowed"
        )));
    }
    Ok(format!("\"{name}\""))
}

/// Split `"schema.table"` → `("schema", "table")`, defaulting schema to `"public"`.
pub fn split_schema_table(name: &str) -> (String, String) {
    match name.split_once('.') {
        Some((s, t)) => (s.to_string(), t.to_string()),
        None => ("public".to_string(), name.to_string()),
    }
}
