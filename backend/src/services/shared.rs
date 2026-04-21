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
        "INSERT INTO audit_logs (action, resource_type, resource_id, message) \
         VALUES ($1, $2, $3, $4)",
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

// ── SQL identifier safety ─────────────────────────────────────────────────────

/// Validate that an identifier contains only `[a-zA-Z0-9_]`, is 1–63 chars,
/// and does not start with a digit.
pub fn ensure_safe_ident(name: &str) -> Result<(), AppError> {
    if name.is_empty() || name.len() > 63 {
        return Err(AppError::BadRequest(format!(
            "invalid identifier: {name:?} — must be 1–63 chars"
        )));
    }
    if name.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return Err(AppError::BadRequest(format!(
            "invalid identifier: {name:?} — must not start with a digit"
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(AppError::BadRequest(format!(
            "identifier has invalid chars: {name:?} — only [a-zA-Z0-9_] allowed"
        )));
    }
    Ok(())
}

// ── Schema name generation ────────────────────────────────────────────────────

/// Generate a safe, unique schema name for a project from its display name.
///
/// Result is always `proj_<slug>` where slug contains only `[a-z0-9_]`.
pub async fn generate_schema_name(pool: &PgPool, name: &str) -> Result<String, AppError> {
    let lowered = name.to_lowercase();
    let mut cleaned = String::new();
    let mut prev_underscore = false;
    for c in lowered.chars() {
        if c.is_ascii_alphanumeric() {
            cleaned.push(c);
            prev_underscore = false;
        } else if !prev_underscore {
            cleaned.push('_');
            prev_underscore = true;
        }
    }
    let cleaned = cleaned.trim_matches('_').to_string();
    let cleaned = if cleaned.is_empty() {
        "unnamed".to_string()
    } else {
        cleaned
    };
    let cleaned: String = cleaned.chars().take(58).collect();

    let candidate = format!("proj_{cleaned}");

    let mut final_name = candidate.clone();
    let mut suffix = 2u32;
    loop {
        let exists: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM _miransas.projects WHERE schema_name = $1",
        )
        .bind(&final_name)
        .fetch_optional(pool)
        .await?;

        if exists.is_none() {
            break;
        }
        final_name = format!("{}_{}", candidate, suffix);
        suffix += 1;
    }

    Ok(final_name)
}

// ── DB helpers ────────────────────────────────────────────────────────────────

/// Look up a project's schema_name. Returns NotFound if project doesn't exist.
pub async fn get_schema_name(pool: &PgPool, project_id: Uuid) -> Result<String, AppError> {
    sqlx::query_scalar("SELECT schema_name FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("project {project_id} not found")))
}
