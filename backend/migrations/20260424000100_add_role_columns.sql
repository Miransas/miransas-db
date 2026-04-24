ALTER TABLE _miransas.projects
  ADD COLUMN IF NOT EXISTS db_role TEXT,
  ADD COLUMN IF NOT EXISTS db_password_encrypted TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS projects_db_role_unique
  ON _miransas.projects(db_role) WHERE db_role IS NOT NULL;
