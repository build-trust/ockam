CREATE TABLE _rust_migrations
(
    name              TEXT NOT NULL,
    run_on            TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX name_index ON _rust_migrations (name);
