use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{CreateSavedQueryRequest, SavedQuery, UpdateSavedQueryRequest},
};

use super::shared::{insert_audit_log, required_text};

pub async fn list(pool: &PgPool, project_id: Uuid) -> Result<Vec<SavedQuery>, AppError> {
    let rows = sqlx::query_as::<_, SavedQuery>(
        "SELECT id, project_id, name, sql, notes, created_at, updated_at \
         FROM saved_queries WHERE project_id = $1 ORDER BY updated_at DESC",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get(pool: &PgPool, id: Uuid) -> Result<SavedQuery, AppError> {
    sqlx::query_as::<_, SavedQuery>(
        "SELECT id, project_id, name, sql, notes, created_at, updated_at \
         FROM saved_queries WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("saved query {id} not found")))
}

pub async fn create(
    pool: &PgPool,
    project_id: Uuid,
    input: CreateSavedQueryRequest,
) -> Result<SavedQuery, AppError> {
    let name = required_text("name", input.name)?;
    let sql = required_text("sql", input.sql)?;

    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects WHERE id = $1)")
            .bind(project_id)
            .fetch_one(pool)
            .await?;
    if !exists {
        return Err(AppError::NotFound(format!("project {project_id} not found")));
    }

    let saved = sqlx::query_as::<_, SavedQuery>(
        "INSERT INTO saved_queries (project_id, name, sql, notes) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id, project_id, name, sql, notes, created_at, updated_at",
    )
    .bind(project_id)
    .bind(&name)
    .bind(&sql)
    .bind(input.notes.as_deref())
    .fetch_one(pool)
    .await?;

    insert_audit_log(
        pool,
        "create",
        "saved_query",
        Some(saved.id),
        Some(format!("saved query '{}' in project {}", name, project_id)),
    )
    .await?;

    Ok(saved)
}

pub async fn update(
    pool: &PgPool,
    id: Uuid,
    input: UpdateSavedQueryRequest,
) -> Result<SavedQuery, AppError> {
    if let Some(ref n) = input.name {
        required_text("name", n.clone())?;
    }
    if let Some(ref s) = input.sql {
        required_text("sql", s.clone())?;
    }

    let has_name = input.name.is_some();
    let name_val = input.name.clone().unwrap_or_default();
    let has_sql = input.sql.is_some();
    let sql_val = input.sql.clone().unwrap_or_default();
    let has_notes = input.notes.is_some();
    let notes_val = input.notes.clone().unwrap_or_default();

    let saved = sqlx::query_as::<_, SavedQuery>(
        r#"
        UPDATE saved_queries
        SET    name       = CASE WHEN $2 THEN $3            ELSE name  END,
               sql        = CASE WHEN $4 THEN $5            ELSE sql   END,
               notes      = CASE WHEN $6 THEN NULLIF($7, '') ELSE notes END,
               updated_at = NOW()
        WHERE  id = $1
        RETURNING id, project_id, name, sql, notes, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(has_name)
    .bind(name_val)
    .bind(has_sql)
    .bind(sql_val)
    .bind(has_notes)
    .bind(notes_val)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("saved query {id} not found")))?;

    insert_audit_log(
        pool,
        "update",
        "saved_query",
        Some(id),
        Some(format!("updated saved query {id}")),
    )
    .await?;

    Ok(saved)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let exists: Option<(i32,)> =
        sqlx::query_as("SELECT 1 FROM saved_queries WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;
    if exists.is_none() {
        return Err(AppError::NotFound(format!("saved query {id} not found")));
    }

    sqlx::query("DELETE FROM saved_queries WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    insert_audit_log(
        pool,
        "delete",
        "saved_query",
        Some(id),
        Some(format!("deleted saved query {id}")),
    )
    .await?;

    Ok(())
}
