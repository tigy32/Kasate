-- Kasate extension SQL installation script
-- This script sets up the table access method

-- The handler function and access method are created by the Rust code via pg_extern

-- Example usage:
-- CREATE TABLE test_table (
--     id SERIAL PRIMARY KEY,
--     name TEXT NOT NULL,
--     value INTEGER
-- ) USING kasate;

-- Helper function to check if a table is using Kasate
CREATE OR REPLACE FUNCTION is_using_kasate(table_name text)
RETURNS boolean AS $$
DECLARE
    am_name text;
BEGIN
    SELECT a.amname INTO am_name
    FROM pg_class c
    JOIN pg_am a ON c.relam = a.oid
    WHERE c.relname = table_name;

    RETURN am_name = 'kasate';
END;
$$ LANGUAGE plpgsql;

-- Function to convert an existing table to use Kasate
-- Note: This will lose all existing data!
CREATE OR REPLACE FUNCTION convert_to_kasate(table_name text)
RETURNS void AS $$
BEGIN
    EXECUTE format('ALTER TABLE %I SET ACCESS METHOD kasate', table_name);
END;
$$ LANGUAGE plpgsql;
