-- Test data for Kunger CLI TUI testing
-- This script creates sample software inventory data for testing

-- First, we need to check the schema from the actual implementation
-- For now, this is a template - adjust based on actual schema

-- Example insert statements (schema may vary):
-- This assumes the following tables exist:
-- - scans (id, completed_at, provider_status_json)
-- - items (id, scan_id, package_id, display_name, description, version, etc.)

-- Note: The actual schema is defined in src/persistence/mod.rs
-- Adjust these statements to match your actual schema

BEGIN TRANSACTION;

-- Create scans table if not exists (example schema)
CREATE TABLE IF NOT EXISTS scans (
    id INTEGER PRIMARY KEY,
    completed_at TEXT NOT NULL,
    provider_status_json TEXT NOT NULL
);

-- Insert a test scan
INSERT INTO scans (completed_at, provider_status_json) VALUES (
    datetime('now'),
    '{}'
);

-- Sample data would go here based on actual schema
-- The tests create in-memory data, so this is mainly for CLI testing

COMMIT;

-- For CLI testing, you'll want to populate with actual software items
-- This can be done by:
-- 1. Running a real scan using the desktop app
-- 2. Using a helper script to generate realistic data
-- 3. Importing from /etc/apt/sources.list, dpkg database, etc.
