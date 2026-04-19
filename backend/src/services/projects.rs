use sqlx::PgPool;

use crate::{
    errors::AppError,
    models::{CreateProjectRequest, Project},
};

use super::shared::{empty_to_none, insert_audit_log, required_text};

pub async fn list_projects(pool: &PgPool) -> Result<Vec<Project>, AppError> {
    let projects = sqlx::query_as::<_, Project>(
        r#"
        SELECT id, name, description, repository_url, created_at, updated_at
        FROM projects
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(projects)
}

pub async fn create_project(
    pool: &PgPool,
    input: CreateProjectRequest,
) -> Result<Project, AppError> {
    let name = required_text("name", input.name)?;

    let project = sqlx::query_as::<_, Project>(
        r#"
        INSERT INTO projects (name, description, repository_url)
        VALUES ($1, $2, $3)
        RETURNING id, name, description, repository_url, created_at, updated_at
        "#,
    )
    .bind(name)
    .bind(empty_to_none(input.description))
    .bind(empty_to_none(input.repository_url))
    .fetch_one(pool)
    .await?;

    insert_audit_log(
        pool,
        "create",
        "project",
        Some(project.id),
        Some(format!("created project {}", project.name)),
    )
    .await?;

    Ok(project)
}
