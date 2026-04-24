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
    pub schema_name: String,
    pub db_role: Option<String>,
    #[serde(skip_serializing)]
    pub db_password_encrypted: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ConnectionInfo {
    pub role: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub schema: String,
    pub connection_string: String,
    pub psql_command: String,
    pub env_snippet: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectResetPasswordResponse {
    pub role: String,
    pub password: String,
    pub connection_string: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub repository_url: Option<String>,
}

/// All fields are optional. Absent = keep existing. Empty string = set to NULL.
#[derive(Debug, Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub repository_url: Option<String>,
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
    pub secret_count: i64,
    pub audit_log_count: i64,
    pub generated_at: DateTime<Utc>,
}

// ── User management ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ProjectUserConfig {
    pub project_id: Uuid,
    pub users_table: String,
    pub id_column: String,
    pub email_column: Option<String>,
    pub username_column: Option<String>,
    pub password_column: Option<String>,
    pub banned_column: Option<String>,
    pub password_algorithm: String,
    pub searchable_columns: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct PutUserConfigRequest {
    pub users_table: String,
    pub id_column: Option<String>,
    pub email_column: Option<String>,
    pub username_column: Option<String>,
    pub password_column: Option<String>,
    pub banned_column: Option<String>,
    pub password_algorithm: Option<String>,
    pub searchable_columns: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UserSearchQuery {
    pub q: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    /// "csv" or "json" (default "json")
    pub format: Option<String>,
    pub max_rows: Option<i64>,
}

// ── Saved queries ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SavedQuery {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub sql: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSavedQueryRequest {
    pub name: String,
    pub sql: String,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSavedQueryRequest {
    pub name: Option<String>,
    pub sql: Option<String>,
    pub notes: Option<String>,
}

// ── Schema editor ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ColumnSpec {
    pub name: String,
    pub data_type: String,
    pub nullable: Option<bool>,
    pub default_value: Option<String>,
    pub primary_key: Option<bool>,
    pub unique: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTableRequest {
    pub name: String,
    pub columns: Vec<ColumnSpec>,
    pub if_not_exists: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AddColumnRequest {
    pub column: ColumnSpec,
}

#[derive(Debug, Deserialize)]
pub struct RenameColumnRequest {
    pub new_name: String,
}

#[derive(Debug, Deserialize)]
pub struct AlterColumnTypeRequest {
    pub new_type: String,
    pub using: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RenameTableRequest {
    pub new_name: String,
}

#[derive(Debug, Deserialize)]
pub struct AddForeignKeyRequest {
    pub constraint_name: String,
    pub column: String,
    pub references_table: String,
    pub references_column: String,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateIndexRequest {
    pub index_name: String,
    pub columns: Vec<String>,
    pub unique: Option<bool>,
    pub method: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddCheckConstraintRequest {
    pub constraint_name: String,
    pub expression: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ColumnInfo {
    pub column_name: String,
    pub data_type: String,
    pub is_nullable: String,
    pub column_default: Option<String>,
    pub character_maximum_length: Option<i32>,
    pub ordinal_position: i32,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ConstraintInfo {
    pub constraint_name: String,
    pub constraint_type: String,
    pub column_names: Option<String>,
    pub foreign_table: Option<String>,
    pub foreign_columns: Option<String>,
    pub check_clause: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct IndexInfo {
    pub index_name: String,
    pub column_names: Option<String>,
    pub is_unique: bool,
    pub index_method: String,
}

#[derive(Debug, Serialize)]
pub struct TableStructureResponse {
    pub schema: String,
    pub table: String,
    pub columns: Vec<ColumnInfo>,
    pub constraints: Vec<ConstraintInfo>,
    pub indexes: Vec<IndexInfo>,
}

#[derive(Debug, Deserialize, Default)]
pub struct DropTableQuery {
    pub cascade: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct DropColumnQuery {
    pub cascade: Option<bool>,
}

// ── Query history ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct QueryHistoryEntry {
    pub id: Uuid,
    pub project_id: Uuid,
    pub sql: String,
    pub duration_ms: i32,
    pub rows_affected: Option<i64>,
    pub success: bool,
    pub error_message: Option<String>,
    pub executed_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct QueryHistoryFilter {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub success: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct QueryHistoryResponse {
    pub rows: Vec<QueryHistoryEntry>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}
