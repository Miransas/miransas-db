use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Projects ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub repository_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub repository_url: Option<String>,
    /// Postgres connection string, e.g. `postgres://user:pass@host:5432/db`.
    /// Stored AES-256-GCM encrypted; never returned in API responses.
    pub connection_string: Option<String>,
}

/// All fields are optional. Absent = keep existing. Empty string = set to NULL.
#[derive(Debug, Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub repository_url: Option<String>,
    /// Empty string clears the stored connection string; non-empty re-encrypts.
    pub connection_string: Option<String>,
}

/// All fields are optional. Absent = keep existing. Empty string = set to NULL.
#[derive(Debug, Deserialize)]
pub struct UpdateDatabaseRequest {
    pub name: Option<String>,
    pub engine: Option<String>,
    pub project_id: Option<Uuid>,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub database_name: Option<String>,
    pub username: Option<String>,
    pub notes: Option<String>,
    /// Empty string clears the stored connection URL; non-empty re-encrypts.
    pub connection_url: Option<String>,
}

// ── Databases ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DatabaseMetadata {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub name: String,
    pub engine: String,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub database_name: Option<String>,
    pub username: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDatabaseRequest {
    pub project_id: Option<Uuid>,
    pub name: String,
    pub engine: String,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub database_name: Option<String>,
    pub username: Option<String>,
    pub notes: Option<String>,
    /// Full connection URL stored encrypted; never returned in responses.
    pub connection_url: Option<String>,
}

// ── Table exploration ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TableInfo {
    pub schema: String,
    pub name: String,
    pub table_type: String,
}

/// Query params for paginated table data.
/// Accepts both `page_size` and `limit` (alias) so callers can use either.
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    /// Alias for `page_size` — whichever is set wins; `page_size` takes precedence.
    pub limit: Option<i64>,
}

impl PaginationQuery {
    pub fn resolved_page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn resolved_limit(&self) -> i64 {
        self.page_size.or(self.limit).unwrap_or(50).clamp(1, 200)
    }
}

#[derive(Debug, Serialize)]
pub struct TableDataResponse {
    pub columns: Vec<String>,
    pub rows: Vec<serde_json::Value>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

// ── Raw query ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub sql: String,
}

#[derive(Debug, Serialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<serde_json::Value>,
    pub rows_affected: Option<u64>,
    pub message: String,
}

// ── Row delete ────────────────────────────────────────────────────────────────

/// Optional query params for the DELETE /projects/:id/tables/:table/:row_id route.
#[derive(Debug, Deserialize)]
pub struct DeleteRowQuery {
    /// Name of the primary-key column. Defaults to `"id"` if omitted.
    pub pk: Option<String>,
}

// ── Secrets ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SecretMetadata {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub name: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSecretRequest {
    pub project_id: Option<Uuid>,
    pub name: String,
    pub value: String,
    pub notes: Option<String>,
}

/// Returned by GET /api/secrets/:id/reveal — includes the decrypted plaintext value.
#[derive(Debug, Serialize)]
pub struct SecretWithValue {
    pub id: Uuid,
    pub name: String,
    pub value: String,
    pub notes: Option<String>,
    pub project_id: Option<Uuid>,
}

// ── Audit / Admin ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Query params for GET /api/audit-logs
#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub resource_type: Option<String>,
    pub resource_id: Option<Uuid>,
}

impl AuditLogQuery {
    pub fn resolved_page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn resolved_limit(&self) -> i64 {
        self.limit.unwrap_or(50).clamp(1, 200)
    }
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AdminSummary {
    pub project_count: i64,
    pub database_count: i64,
    pub secret_count: i64,
    pub audit_log_count: i64,
    pub generated_at: DateTime<Utc>,
}
