use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Projects ─────────────────────────────────────────────────────────────────

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
    /// Full connection URL (e.g. postgres://user:pass@host:5432/db).
    /// Stored encrypted; never returned in responses.
    pub connection_url: Option<String>,
}

// ── Table exploration ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TableInfo {
    pub schema: String,
    pub name: String,
    pub table_type: String,
}

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
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
