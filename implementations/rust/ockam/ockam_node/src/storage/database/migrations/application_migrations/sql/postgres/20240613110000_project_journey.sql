CREATE TABLE project_journey
(
    tenant_id                      TEXT NOT NULL,
    project_id                     TEXT NOT NULL,
    opentelemetry_context          TEXT NOT NULL UNIQUE,
    start_datetime                 TEXT NOT NULL,
    previous_opentelemetry_context TEXT
);

CREATE TABLE host_journey
(
    tenant_id                      TEXT NOT NULL,
    opentelemetry_context          TEXT NOT NULL UNIQUE,
    start_datetime                 TEXT NOT NULL,
    previous_opentelemetry_context TEXT
);
