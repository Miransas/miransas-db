-- Projects now carry their own encrypted connection string so they can be
-- used directly as a database target (list tables, run queries, etc.).
ALTER TABLE projects
    ADD COLUMN IF NOT EXISTS connection_string_encrypted TEXT;
