REVOKE ALL ON SCHEMA _miransas FROM PUBLIC;
REVOKE ALL ON SCHEMA public FROM PUBLIC;

-- Keep CONNECT so roles can log in to the database at all.
DO $$
DECLARE
    db text := current_database();
BEGIN
    EXECUTE format('GRANT CONNECT ON DATABASE %I TO PUBLIC', db);
END $$;
