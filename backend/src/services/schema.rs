use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{
        AddCheckConstraintRequest, AddColumnRequest, AddForeignKeyRequest, AlterColumnTypeRequest,
        ColumnInfo, ColumnSpec, ConstraintInfo, CreateIndexRequest, CreateTableRequest,
        IndexInfo, RenameColumnRequest, RenameTableRequest, TableStructureResponse,
    },
};

use super::shared::{
    ensure_safe_ident, get_schema_name, insert_audit_log, validate_data_type, validate_fk_action,
    validate_index_method,
};

// ── Security helpers ──────────────────────────────────────────────────────────

fn validate_default_expression(expr: &str) -> Result<(), AppError> {
    let trimmed = expr.trim();
    if trimmed.is_empty() || trimmed.len() > 500 {
        return Err(AppError::BadRequest(
            "default/using/check expression must be 1-500 chars".to_string(),
        ));
    }
    if trimmed.contains(';') {
        return Err(AppError::BadRequest(
            "expression cannot contain semicolon".to_string(),
        ));
    }
    Ok(())
}

// ── Column fragment builder ───────────────────────────────────────────────────

fn build_column_fragment(col: &ColumnSpec) -> Result<String, AppError> {
    ensure_safe_ident(&col.name)?;
    let validated_type = validate_data_type(&col.data_type)?;
    let mut frag = format!("\"{}\" {}", col.name, validated_type);

    if !col.nullable.unwrap_or(true) {
        frag.push_str(" NOT NULL");
    }
    if col.primary_key.unwrap_or(false) {
        frag.push_str(" PRIMARY KEY");
    }
    if col.unique.unwrap_or(false) {
        frag.push_str(" UNIQUE");
    }
    if let Some(ref def) = col.default_value {
        validate_default_expression(def)?;
        frag.push_str(&format!(" DEFAULT {}", def));
    }

    Ok(frag)
}

// ── Tables ────────────────────────────────────────────────────────────────────

pub async fn create_table(
    pool: &PgPool,
    project_id: Uuid,
    input: CreateTableRequest,
) -> Result<(), AppError> {
    ensure_safe_ident(&input.name)?;
    if input.columns.is_empty() {
        return Err(AppError::BadRequest(
            "columns must not be empty".to_string(),
        ));
    }
    let schema = get_schema_name(pool, project_id).await?;
    ensure_safe_ident(&schema)?;

    let mut frags = Vec::with_capacity(input.columns.len());
    for col in &input.columns {
        frags.push(build_column_fragment(col)?);
    }

    let if_not_exists = if input.if_not_exists.unwrap_or(false) {
        "IF NOT EXISTS "
    } else {
        ""
    };

    let sql = format!(
        "CREATE TABLE {}\"{}\".\"{}\"\u{0020}({})",
        if_not_exists,
        schema,
        input.name,
        frags.join(", ")
    );

    sqlx::query(&sql).execute(pool).await?;

    insert_audit_log(
        pool,
        "create_table",
        "schema",
        Some(project_id),
        Some(format!("created table {}.{}", schema, input.name)),
    )
    .await?;

    Ok(())
}

pub async fn drop_table(
    pool: &PgPool,
    project_id: Uuid,
    table: &str,
    cascade: bool,
) -> Result<(), AppError> {
    ensure_safe_ident(table)?;
    let schema = get_schema_name(pool, project_id).await?;
    ensure_safe_ident(&schema)?;

    let sql = format!(
        "DROP TABLE \"{}\".\"{}\"{}",
        schema,
        table,
        if cascade { " CASCADE" } else { "" }
    );

    sqlx::query(&sql).execute(pool).await?;

    insert_audit_log(
        pool,
        "drop_table",
        "schema",
        Some(project_id),
        Some(format!("dropped table {}.{}", schema, table)),
    )
    .await?;

    Ok(())
}

pub async fn rename_table(
    pool: &PgPool,
    project_id: Uuid,
    table: &str,
    input: RenameTableRequest,
) -> Result<(), AppError> {
    ensure_safe_ident(table)?;
    ensure_safe_ident(&input.new_name)?;
    let schema = get_schema_name(pool, project_id).await?;
    ensure_safe_ident(&schema)?;

    let sql = format!(
        "ALTER TABLE \"{}\".\"{}\" RENAME TO \"{}\"",
        schema, table, input.new_name
    );

    sqlx::query(&sql).execute(pool).await?;

    insert_audit_log(
        pool,
        "rename_table",
        "schema",
        Some(project_id),
        Some(format!("renamed table {}.{} to {}", schema, table, input.new_name)),
    )
    .await?;

    Ok(())
}

// ── Columns ───────────────────────────────────────────────────────────────────

pub async fn add_column(
    pool: &PgPool,
    project_id: Uuid,
    table: &str,
    input: AddColumnRequest,
) -> Result<(), AppError> {
    ensure_safe_ident(table)?;
    let schema = get_schema_name(pool, project_id).await?;
    ensure_safe_ident(&schema)?;

    let frag = build_column_fragment(&input.column)?;
    let sql = format!(
        "ALTER TABLE \"{}\".\"{}\" ADD COLUMN {}",
        schema, table, frag
    );

    sqlx::query(&sql).execute(pool).await?;

    insert_audit_log(
        pool,
        "add_column",
        "schema",
        Some(project_id),
        Some(format!(
            "added column {} to {}.{}",
            input.column.name, schema, table
        )),
    )
    .await?;

    Ok(())
}

pub async fn drop_column(
    pool: &PgPool,
    project_id: Uuid,
    table: &str,
    column: &str,
    cascade: bool,
) -> Result<(), AppError> {
    ensure_safe_ident(table)?;
    ensure_safe_ident(column)?;
    let schema = get_schema_name(pool, project_id).await?;
    ensure_safe_ident(&schema)?;

    let sql = format!(
        "ALTER TABLE \"{}\".\"{}\" DROP COLUMN \"{}\"{}",
        schema,
        table,
        column,
        if cascade { " CASCADE" } else { "" }
    );

    sqlx::query(&sql).execute(pool).await?;

    insert_audit_log(
        pool,
        "drop_column",
        "schema",
        Some(project_id),
        Some(format!("dropped column {} from {}.{}", column, schema, table)),
    )
    .await?;

    Ok(())
}

pub async fn rename_column(
    pool: &PgPool,
    project_id: Uuid,
    table: &str,
    column: &str,
    input: RenameColumnRequest,
) -> Result<(), AppError> {
    ensure_safe_ident(table)?;
    ensure_safe_ident(column)?;
    ensure_safe_ident(&input.new_name)?;
    let schema = get_schema_name(pool, project_id).await?;
    ensure_safe_ident(&schema)?;

    let sql = format!(
        "ALTER TABLE \"{}\".\"{}\" RENAME COLUMN \"{}\" TO \"{}\"",
        schema, table, column, input.new_name
    );

    sqlx::query(&sql).execute(pool).await?;

    insert_audit_log(
        pool,
        "rename_column",
        "schema",
        Some(project_id),
        Some(format!(
            "renamed column {} to {} in {}.{}",
            column, input.new_name, schema, table
        )),
    )
    .await?;

    Ok(())
}

pub async fn alter_column_type(
    pool: &PgPool,
    project_id: Uuid,
    table: &str,
    column: &str,
    input: AlterColumnTypeRequest,
) -> Result<(), AppError> {
    ensure_safe_ident(table)?;
    ensure_safe_ident(column)?;
    let schema = get_schema_name(pool, project_id).await?;
    ensure_safe_ident(&schema)?;

    let new_type = validate_data_type(&input.new_type)?;

    let using_clause = if let Some(ref u) = input.using {
        validate_default_expression(u)?;
        format!(" USING {}", u)
    } else {
        String::new()
    };

    let sql = format!(
        "ALTER TABLE \"{}\".\"{}\" ALTER COLUMN \"{}\" TYPE {}{}",
        schema, table, column, new_type, using_clause
    );

    sqlx::query(&sql).execute(pool).await?;

    insert_audit_log(
        pool,
        "alter_column_type",
        "schema",
        Some(project_id),
        Some(format!(
            "changed type of {}.{}.{} to {}",
            schema, table, column, new_type
        )),
    )
    .await?;

    Ok(())
}

// ── Foreign keys ──────────────────────────────────────────────────────────────

pub async fn add_foreign_key(
    pool: &PgPool,
    project_id: Uuid,
    table: &str,
    input: AddForeignKeyRequest,
) -> Result<(), AppError> {
    ensure_safe_ident(table)?;
    ensure_safe_ident(&input.constraint_name)?;
    ensure_safe_ident(&input.column)?;
    ensure_safe_ident(&input.references_table)?;
    ensure_safe_ident(&input.references_column)?;
    let schema = get_schema_name(pool, project_id).await?;
    ensure_safe_ident(&schema)?;

    let on_delete = input
        .on_delete
        .as_deref()
        .map(validate_fk_action)
        .transpose()?;
    let on_update = input
        .on_update
        .as_deref()
        .map(validate_fk_action)
        .transpose()?;

    let mut parts = vec![
        format!("ALTER TABLE \"{}\".\"{}\"", schema, table),
        format!("ADD CONSTRAINT \"{}\"", input.constraint_name),
        format!("FOREIGN KEY (\"{}\")", input.column),
        format!(
            "REFERENCES \"{}\".\"{}\"\u{0020}(\"{}\")",
            schema, input.references_table, input.references_column
        ),
    ];
    if let Some(a) = on_delete {
        parts.push(format!("ON DELETE {}", a));
    }
    if let Some(a) = on_update {
        parts.push(format!("ON UPDATE {}", a));
    }
    let sql = parts.join(" ");

    sqlx::query(&sql).execute(pool).await?;

    insert_audit_log(
        pool,
        "add_foreign_key",
        "schema",
        Some(project_id),
        Some(format!(
            "added FK {} on {}.{}({})",
            input.constraint_name, schema, table, input.column
        )),
    )
    .await?;

    Ok(())
}

pub async fn drop_constraint(
    pool: &PgPool,
    project_id: Uuid,
    table: &str,
    constraint_name: &str,
) -> Result<(), AppError> {
    ensure_safe_ident(table)?;
    ensure_safe_ident(constraint_name)?;
    let schema = get_schema_name(pool, project_id).await?;
    ensure_safe_ident(&schema)?;

    let sql = format!(
        "ALTER TABLE \"{}\".\"{}\" DROP CONSTRAINT \"{}\"",
        schema, table, constraint_name
    );

    sqlx::query(&sql).execute(pool).await?;

    insert_audit_log(
        pool,
        "drop_constraint",
        "schema",
        Some(project_id),
        Some(format!(
            "dropped constraint {} from {}.{}",
            constraint_name, schema, table
        )),
    )
    .await?;

    Ok(())
}

// ── Check constraints ─────────────────────────────────────────────────────────

pub async fn add_check_constraint(
    pool: &PgPool,
    project_id: Uuid,
    table: &str,
    input: AddCheckConstraintRequest,
) -> Result<(), AppError> {
    ensure_safe_ident(table)?;
    ensure_safe_ident(&input.constraint_name)?;
    validate_default_expression(&input.expression)?;
    let schema = get_schema_name(pool, project_id).await?;
    ensure_safe_ident(&schema)?;

    let sql = format!(
        "ALTER TABLE \"{}\".\"{}\" ADD CONSTRAINT \"{}\" CHECK ({})",
        schema, table, input.constraint_name, input.expression
    );

    sqlx::query(&sql).execute(pool).await?;

    insert_audit_log(
        pool,
        "add_check_constraint",
        "schema",
        Some(project_id),
        Some(format!(
            "added check constraint {} on {}.{}",
            input.constraint_name, schema, table
        )),
    )
    .await?;

    Ok(())
}

// ── Indexes ───────────────────────────────────────────────────────────────────

pub async fn create_index(
    pool: &PgPool,
    project_id: Uuid,
    table: &str,
    input: CreateIndexRequest,
) -> Result<(), AppError> {
    ensure_safe_ident(table)?;
    ensure_safe_ident(&input.index_name)?;
    if input.columns.is_empty() {
        return Err(AppError::BadRequest("columns must not be empty".to_string()));
    }
    for col in &input.columns {
        ensure_safe_ident(col)?;
    }
    let schema = get_schema_name(pool, project_id).await?;
    ensure_safe_ident(&schema)?;

    let method = input
        .method
        .as_deref()
        .map(validate_index_method)
        .transpose()?
        .unwrap_or_else(|| "BTREE".to_string());

    let cols_quoted = input
        .columns
        .iter()
        .map(|c| format!("\"{}\"", c))
        .collect::<Vec<_>>()
        .join(", ");

    let unique_keyword = if input.unique.unwrap_or(false) {
        "UNIQUE "
    } else {
        ""
    };

    let sql = format!(
        "CREATE {}INDEX \"{}\" ON \"{}\".\"{}\"\u{0020}USING {} ({})",
        unique_keyword, input.index_name, schema, table, method, cols_quoted
    );

    sqlx::query(&sql).execute(pool).await?;

    insert_audit_log(
        pool,
        "create_index",
        "schema",
        Some(project_id),
        Some(format!(
            "created index {} on {}.{}",
            input.index_name, schema, table
        )),
    )
    .await?;

    Ok(())
}

pub async fn drop_index(
    pool: &PgPool,
    project_id: Uuid,
    index_name: &str,
) -> Result<(), AppError> {
    ensure_safe_ident(index_name)?;
    let schema = get_schema_name(pool, project_id).await?;
    ensure_safe_ident(&schema)?;

    let sql = format!("DROP INDEX \"{}\".\"{}\"", schema, index_name);

    sqlx::query(&sql).execute(pool).await?;

    insert_audit_log(
        pool,
        "drop_index",
        "schema",
        Some(project_id),
        Some(format!("dropped index {} in schema {}", index_name, schema)),
    )
    .await?;

    Ok(())
}

// ── Introspection ─────────────────────────────────────────────────────────────

pub async fn get_table_structure(
    pool: &PgPool,
    project_id: Uuid,
    table: &str,
) -> Result<TableStructureResponse, AppError> {
    ensure_safe_ident(table)?;
    let schema = get_schema_name(pool, project_id).await?;
    ensure_safe_ident(&schema)?;

    let exists: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM information_schema.tables \
         WHERE table_schema = $1 AND table_name = $2",
    )
    .bind(&schema)
    .bind(table)
    .fetch_optional(pool)
    .await?;

    if exists.is_none() {
        return Err(AppError::NotFound(format!(
            "table {}.{} not found",
            schema, table
        )));
    }

    let columns: Vec<ColumnInfo> = sqlx::query_as(
        "SELECT column_name, data_type, is_nullable, column_default, \
                character_maximum_length, ordinal_position \
         FROM information_schema.columns \
         WHERE table_schema = $1 AND table_name = $2 \
         ORDER BY ordinal_position",
    )
    .bind(&schema)
    .bind(table)
    .fetch_all(pool)
    .await?;

    let constraints: Vec<ConstraintInfo> = sqlx::query_as(
        "SELECT
            con.conname AS constraint_name,
            CASE con.contype
              WHEN 'p' THEN 'PRIMARY KEY'
              WHEN 'u' THEN 'UNIQUE'
              WHEN 'f' THEN 'FOREIGN KEY'
              WHEN 'c' THEN 'CHECK'
              ELSE con.contype::text
            END AS constraint_type,
            (SELECT string_agg(att.attname, ',' ORDER BY array_position(con.conkey, att.attnum))
             FROM unnest(con.conkey) WITH ORDINALITY AS k(attnum, ord)
             JOIN pg_attribute att ON att.attrelid = con.conrelid AND att.attnum = k.attnum
            ) AS column_names,
            (SELECT c2.relname FROM pg_class c2 WHERE c2.oid = con.confrelid) AS foreign_table,
            (SELECT string_agg(att.attname, ',' ORDER BY array_position(con.confkey, att.attnum))
             FROM unnest(con.confkey) WITH ORDINALITY AS k(attnum, ord)
             JOIN pg_attribute att ON att.attrelid = con.confrelid AND att.attnum = k.attnum
            ) AS foreign_columns,
            pg_get_constraintdef(con.oid) AS check_clause
         FROM pg_constraint con
         JOIN pg_class cls ON cls.oid = con.conrelid
         JOIN pg_namespace ns ON ns.oid = cls.relnamespace
         WHERE ns.nspname = $1 AND cls.relname = $2",
    )
    .bind(&schema)
    .bind(table)
    .fetch_all(pool)
    .await?;

    let indexes: Vec<IndexInfo> = sqlx::query_as(
        "SELECT
            i.relname AS index_name,
            (SELECT string_agg(a.attname, ',' ORDER BY array_position(idx.indkey::int[], a.attnum::int))
             FROM unnest(idx.indkey::int[]) WITH ORDINALITY AS k(attnum, ord)
             JOIN pg_attribute a ON a.attrelid = idx.indrelid AND a.attnum = k.attnum
             WHERE k.attnum > 0
            ) AS column_names,
            idx.indisunique AS is_unique,
            am.amname AS index_method
         FROM pg_index idx
         JOIN pg_class i ON i.oid = idx.indexrelid
         JOIN pg_class c ON c.oid = idx.indrelid
         JOIN pg_namespace ns ON ns.oid = c.relnamespace
         JOIN pg_am am ON am.oid = i.relam
         WHERE ns.nspname = $1 AND c.relname = $2",
    )
    .bind(&schema)
    .bind(table)
    .fetch_all(pool)
    .await?;

    Ok(TableStructureResponse {
        schema,
        table: table.to_string(),
        columns,
        constraints,
        indexes,
    })
}
