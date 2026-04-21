use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{
        ExportQuery, ProjectUserConfig, PutUserConfigRequest, ResetPasswordRequest,
        TableDataResponse, UserSearchQuery,
    },
};

use super::shared::{ensure_safe_ident, get_schema_name, insert_audit_log, required_text};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn rows_as_json(raw: Vec<String>) -> Vec<serde_json::Value> {
    raw.iter()
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect()
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

// ── Config ────────────────────────────────────────────────────────────────────

pub async fn get_config(pool: &PgPool, project_id: Uuid) -> Result<ProjectUserConfig, AppError> {
    sqlx::query_as::<_, ProjectUserConfig>(
        "SELECT * FROM project_user_config WHERE project_id = $1",
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("user config for project {project_id} not found")))
}

pub async fn put_config(
    pool: &PgPool,
    project_id: Uuid,
    input: PutUserConfigRequest,
) -> Result<ProjectUserConfig, AppError> {
    // Verify project exists
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects WHERE id = $1)")
            .bind(project_id)
            .fetch_one(pool)
            .await?;
    if !exists {
        return Err(AppError::NotFound(format!("project {project_id} not found")));
    }

    // Validate all identifier fields
    let users_table = required_text("users_table", input.users_table)?;
    ensure_safe_ident(&users_table)?;

    let id_column = input
        .id_column
        .as_deref()
        .unwrap_or("id")
        .to_string();
    ensure_safe_ident(&id_column)?;

    if let Some(ref c) = input.email_column {
        if !c.is_empty() {
            ensure_safe_ident(c)?;
        }
    }
    if let Some(ref c) = input.username_column {
        if !c.is_empty() {
            ensure_safe_ident(c)?;
        }
    }
    if let Some(ref c) = input.password_column {
        if !c.is_empty() {
            ensure_safe_ident(c)?;
        }
    }
    if let Some(ref c) = input.banned_column {
        if !c.is_empty() {
            ensure_safe_ident(c)?;
        }
    }

    let algorithm = input
        .password_algorithm
        .as_deref()
        .unwrap_or("bcrypt")
        .to_string();
    if !matches!(algorithm.as_str(), "bcrypt" | "argon2" | "plaintext") {
        return Err(AppError::BadRequest(
            "password_algorithm must be one of: bcrypt, argon2, plaintext".to_string(),
        ));
    }

    let searchable_columns = input.searchable_columns.unwrap_or_default();
    for col in &searchable_columns {
        ensure_safe_ident(col)?;
    }

    let email_column = input
        .email_column
        .filter(|s| !s.is_empty());
    let username_column = input
        .username_column
        .filter(|s| !s.is_empty());
    let password_column = input
        .password_column
        .filter(|s| !s.is_empty());
    let banned_column = input
        .banned_column
        .filter(|s| !s.is_empty());

    let config = sqlx::query_as::<_, ProjectUserConfig>(
        r#"
        INSERT INTO project_user_config
            (project_id, users_table, id_column, email_column, username_column,
             password_column, banned_column, password_algorithm, searchable_columns)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (project_id) DO UPDATE SET
            users_table        = EXCLUDED.users_table,
            id_column          = EXCLUDED.id_column,
            email_column       = EXCLUDED.email_column,
            username_column    = EXCLUDED.username_column,
            password_column    = EXCLUDED.password_column,
            banned_column      = EXCLUDED.banned_column,
            password_algorithm = EXCLUDED.password_algorithm,
            searchable_columns = EXCLUDED.searchable_columns
        RETURNING *
        "#,
    )
    .bind(project_id)
    .bind(&users_table)
    .bind(&id_column)
    .bind(email_column)
    .bind(username_column)
    .bind(password_column)
    .bind(banned_column)
    .bind(&algorithm)
    .bind(&searchable_columns)
    .fetch_one(pool)
    .await?;

    insert_audit_log(
        pool,
        "put_user_config",
        "project",
        Some(project_id),
        Some(format!("configured user management for project {project_id}")),
    )
    .await?;

    Ok(config)
}

// ── Users ─────────────────────────────────────────────────────────────────────

pub async fn list_users(
    pool: &PgPool,
    project_id: Uuid,
    q: UserSearchQuery,
) -> Result<TableDataResponse, AppError> {
    let config = get_config(pool, project_id).await?;
    let schema_name = get_schema_name(pool, project_id).await?;

    ensure_safe_ident(&config.users_table)?;
    let quoted = format!("\"{}\".\"{}\"", schema_name, config.users_table);

    let page = q.page.unwrap_or(1).max(1);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * limit;

    // Build optional WHERE clause for full-text search
    let (where_clause, search_param) =
        if let Some(ref search) = q.q {
            if !config.searchable_columns.is_empty() {
                for col in &config.searchable_columns {
                    ensure_safe_ident(col)?;
                }
                let conditions: Vec<String> = config
                    .searchable_columns
                    .iter()
                    .map(|c| format!("\"{}\"::TEXT ILIKE $1", c))
                    .collect();
                (
                    format!(" WHERE {}", conditions.join(" OR ")),
                    Some(format!("%{}%", search)),
                )
            } else {
                (String::new(), None)
            }
        } else {
            (String::new(), None)
        };

    let total: i64 = if let Some(ref param) = search_param {
        sqlx::query_scalar(&format!(
            "SELECT COUNT(*)::BIGINT FROM {}{}",
            quoted, where_clause
        ))
        .bind(param)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar(&format!("SELECT COUNT(*)::BIGINT FROM {}", quoted))
            .fetch_one(pool)
            .await?
    };

    let raw: Vec<String> = if let Some(ref param) = search_param {
        sqlx::query_scalar(&format!(
            "SELECT row_to_json(_t)::TEXT FROM \
             (SELECT * FROM {}{} LIMIT {} OFFSET {}) _t",
            quoted, where_clause, limit, offset
        ))
        .bind(param)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_scalar(&format!(
            "SELECT row_to_json(_t)::TEXT FROM \
             (SELECT * FROM {} LIMIT {} OFFSET {}) _t",
            quoted, limit, offset
        ))
        .fetch_all(pool)
        .await?
    };

    let columns: Vec<String> = sqlx::query_scalar(
        "SELECT column_name::TEXT FROM information_schema.columns \
         WHERE table_schema = $1 AND table_name = $2 ORDER BY ordinal_position",
    )
    .bind(&schema_name)
    .bind(&config.users_table)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    Ok(TableDataResponse {
        columns,
        rows: rows_as_json(raw),
        total,
        page,
        page_size: limit,
    })
}

pub async fn get_user(
    pool: &PgPool,
    project_id: Uuid,
    user_id: &str,
) -> Result<serde_json::Value, AppError> {
    let config = get_config(pool, project_id).await?;
    let schema_name = get_schema_name(pool, project_id).await?;

    ensure_safe_ident(&config.users_table)?;
    ensure_safe_ident(&config.id_column)?;

    let sql = format!(
        "SELECT row_to_json(t)::TEXT FROM \"{}\".\"{}\" t WHERE \"{}\"::TEXT = $1",
        schema_name, config.users_table, config.id_column
    );

    let raw: Option<String> = sqlx::query_scalar(&sql)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    let json_str = raw.ok_or_else(|| AppError::NotFound(format!("user {user_id} not found")))?;
    serde_json::from_str(&json_str)
        .map_err(|e| AppError::BadRequest(format!("failed to parse user row: {e}")))
}

pub async fn delete_user(
    pool: &PgPool,
    project_id: Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let config = get_config(pool, project_id).await?;
    let schema_name = get_schema_name(pool, project_id).await?;

    ensure_safe_ident(&config.users_table)?;
    ensure_safe_ident(&config.id_column)?;

    let sql = format!(
        "DELETE FROM \"{}\".\"{}\" WHERE \"{}\"::TEXT = $1",
        schema_name, config.users_table, config.id_column
    );
    let result = sqlx::query(&sql).bind(user_id).execute(pool).await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("user {user_id} not found")));
    }

    insert_audit_log(
        pool,
        "delete_user",
        "user",
        None,
        Some(format!(
            "deleted user {user_id} from project {project_id}"
        )),
    )
    .await?;

    Ok(())
}

pub async fn ban_user(
    pool: &PgPool,
    project_id: Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let config = get_config(pool, project_id).await?;
    let banned_column = config.banned_column.as_deref().ok_or_else(|| {
        AppError::BadRequest("banned_column not configured for this project".to_string())
    })?;

    let schema_name = get_schema_name(pool, project_id).await?;
    ensure_safe_ident(&config.users_table)?;
    ensure_safe_ident(&config.id_column)?;
    ensure_safe_ident(banned_column)?;

    let sql = format!(
        "UPDATE \"{}\".\"{}\" SET \"{}\" = TRUE WHERE \"{}\"::TEXT = $1",
        schema_name, config.users_table, banned_column, config.id_column
    );
    let result = sqlx::query(&sql).bind(user_id).execute(pool).await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("user {user_id} not found")));
    }

    insert_audit_log(
        pool,
        "ban_user",
        "user",
        None,
        Some(format!("banned user {user_id} in project {project_id}")),
    )
    .await?;

    Ok(())
}

pub async fn unban_user(
    pool: &PgPool,
    project_id: Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let config = get_config(pool, project_id).await?;
    let banned_column = config.banned_column.as_deref().ok_or_else(|| {
        AppError::BadRequest("banned_column not configured for this project".to_string())
    })?;

    let schema_name = get_schema_name(pool, project_id).await?;
    ensure_safe_ident(&config.users_table)?;
    ensure_safe_ident(&config.id_column)?;
    ensure_safe_ident(banned_column)?;

    let sql = format!(
        "UPDATE \"{}\".\"{}\" SET \"{}\" = FALSE WHERE \"{}\"::TEXT = $1",
        schema_name, config.users_table, banned_column, config.id_column
    );
    let result = sqlx::query(&sql).bind(user_id).execute(pool).await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("user {user_id} not found")));
    }

    insert_audit_log(
        pool,
        "unban_user",
        "user",
        None,
        Some(format!("unbanned user {user_id} in project {project_id}")),
    )
    .await?;

    Ok(())
}

pub async fn reset_password(
    pool: &PgPool,
    project_id: Uuid,
    user_id: &str,
    req: ResetPasswordRequest,
) -> Result<(), AppError> {
    let config = get_config(pool, project_id).await?;
    let password_column = config.password_column.as_deref().ok_or_else(|| {
        AppError::BadRequest("password_column not configured for this project".to_string())
    })?;

    let new_password = required_text("new_password", req.new_password)?;

    let hashed = match config.password_algorithm.as_str() {
        "plaintext" => new_password,
        "bcrypt" => {
            return Err(AppError::BadRequest(
                "bcrypt hashing not yet supported, use plaintext or argon2".to_string(),
            ))
        }
        "argon2" => {
            return Err(AppError::BadRequest(
                "argon2 hashing not yet supported".to_string(),
            ))
        }
        other => {
            return Err(AppError::BadRequest(format!(
                "unknown password_algorithm: {other}"
            )))
        }
    };

    let schema_name = get_schema_name(pool, project_id).await?;
    ensure_safe_ident(&config.users_table)?;
    ensure_safe_ident(&config.id_column)?;
    ensure_safe_ident(password_column)?;

    let sql = format!(
        "UPDATE \"{}\".\"{}\" SET \"{}\" = $2 WHERE \"{}\"::TEXT = $1",
        schema_name, config.users_table, password_column, config.id_column
    );
    let result = sqlx::query(&sql)
        .bind(user_id)
        .bind(&hashed)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("user {user_id} not found")));
    }

    insert_audit_log(
        pool,
        "reset_password",
        "user",
        None,
        Some(format!(
            "reset password for user {user_id} in project {project_id}"
        )),
    )
    .await?;

    Ok(())
}

pub async fn export_users(
    pool: &PgPool,
    project_id: Uuid,
    q: ExportQuery,
) -> Result<(String, &'static str), AppError> {
    let config = get_config(pool, project_id).await?;
    let schema_name = get_schema_name(pool, project_id).await?;

    ensure_safe_ident(&config.users_table)?;
    let quoted = format!("\"{}\".\"{}\"", schema_name, config.users_table);

    let raw: Vec<String> = sqlx::query_scalar(&format!(
        "SELECT row_to_json(_t)::TEXT FROM (SELECT * FROM {}) _t",
        quoted
    ))
    .fetch_all(pool)
    .await?;

    insert_audit_log(
        pool,
        "export_users",
        "project",
        Some(project_id),
        Some(format!("exported users for project {project_id}")),
    )
    .await?;

    let format = q.format.as_deref().unwrap_or("json");

    if format == "csv" {
        let rows = rows_as_json(raw);
        if rows.is_empty() {
            return Ok((String::new(), "text/csv"));
        }

        let columns: Vec<String> = rows
            .first()
            .and_then(|r| r.as_object())
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default();

        let mut csv = String::new();
        csv.push_str(
            &columns
                .iter()
                .map(|c| csv_escape(c))
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');

        for row in &rows {
            if let Some(obj) = row.as_object() {
                let line: Vec<String> = columns
                    .iter()
                    .map(|c| {
                        let val = obj.get(c).unwrap_or(&serde_json::Value::Null);
                        match val {
                            serde_json::Value::Null => String::new(),
                            serde_json::Value::String(s) => csv_escape(s),
                            other => csv_escape(&other.to_string()),
                        }
                    })
                    .collect();
                csv.push_str(&line.join(","));
                csv.push('\n');
            }
        }

        Ok((csv, "text/csv"))
    } else {
        Ok((format!("[{}]", raw.join(",")), "application/json"))
    }
}
