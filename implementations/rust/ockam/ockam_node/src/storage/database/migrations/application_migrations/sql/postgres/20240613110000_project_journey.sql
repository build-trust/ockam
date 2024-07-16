CREATE TABLE project_journey (
    project_id                     TEXT NOT NULL,
    opentelemetry_context          TEXT NOT NULL UNIQUE,
    start_datetime                 TEXT NOT NULL,
    previous_opentelemetry_context TEXT
);

CREATE TABLE host_journey (
    opentelemetry_context          TEXT NOT NULL UNIQUE,
    start_datetime                 TEXT NOT NULL,
    previous_opentelemetry_context TEXT
);
