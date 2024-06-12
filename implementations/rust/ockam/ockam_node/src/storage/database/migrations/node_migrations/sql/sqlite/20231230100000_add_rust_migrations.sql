CREATE TABLE IF NOT EXISTS _rust_migrations
(
    name   TEXT      NOT NULL,
    run_on TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS name_index ON _rust_migrations (name);

-- This migration was renamed from 20240111100000_add_rust_migrations.sql, so let's remove
-- the old entry in the migrations table
DELETE
FROM _sqlx_migrations
WHERE version = 20240111100000;
