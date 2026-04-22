-- Restructure: move all internal tables to _miransas schema,
-- add schema_name to projects, drop databases table,
-- add query_history and saved_queries tables.

CREATE SCHEMA IF NOT EXISTS _miransas;


DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM information_schema.tables 
             WHERE table_schema = 'public' AND table_name = 'projects') THEN
    ALTER TABLE public.projects SET SCHEMA _miransas;
  END IF;
  
  IF EXISTS (SELECT 1 FROM information_schema.tables 
             WHERE table_schema = 'public' AND table_name = 'databases') THEN
    ALTER TABLE public.databases SET SCHEMA _miransas;
  END IF;
  
  IF EXISTS (SELECT 1 FROM information_schema.tables 
             WHERE table_schema = 'public' AND table_name = 'secrets') THEN
    ALTER TABLE public.secrets SET SCHEMA _miransas;
  END IF;
  
  IF EXISTS (SELECT 1 FROM information_schema.tables 
             WHERE table_schema = 'public' AND table_name = 'audit_logs') THEN
    ALTER TABLE public.audit_logs SET SCHEMA _miransas;
  END IF;
END $$;

-- schema_name kolon 
ALTER TABLE _miransas.projects ADD COLUMN IF NOT EXISTS schema_name TEXT;
UPDATE _miransas.projects 
  SET schema_name = 'proj_' || lower(regexp_replace(name, '[^a-zA-Z0-9]+', '_', 'g'))
  WHERE schema_name IS NULL;
ALTER TABLE _miransas.projects ALTER COLUMN schema_name SET NOT NULL;

-- unique constraint 
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'projects_schema_name_unique') THEN
    ALTER TABLE _miransas.projects ADD CONSTRAINT projects_schema_name_unique UNIQUE (schema_name);
  END IF;
END $$;

-- connection_string kolon
ALTER TABLE _miransas.projects DROP COLUMN IF EXISTS connection_string_encrypted;

-- databases table
DROP TABLE IF EXISTS _miransas.databases;

-- query_history
CREATE TABLE IF NOT EXISTS _miransas.query_history (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id UUID NOT NULL REFERENCES _miransas.projects(id) ON DELETE CASCADE,
  sql TEXT NOT NULL,
  duration_ms INTEGER NOT NULL,
  rows_affected BIGINT,
  success BOOLEAN NOT NULL,
  error_message TEXT,
  executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_query_history_project 
  ON _miransas.query_history(project_id, executed_at DESC);

CREATE TABLE IF NOT EXISTS _miransas.saved_queries (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id UUID NOT NULL REFERENCES _miransas.projects(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  sql TEXT NOT NULL,
  notes TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_saved_queries_project 
  ON _miransas.saved_queries(project_id);

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'set_saved_queries_updated_at') THEN
    CREATE TRIGGER set_saved_queries_updated_at
      BEFORE UPDATE ON _miransas.saved_queries
      FOR EACH ROW EXECUTE FUNCTION set_updated_at();
  END IF;
END $$;