CREATE TABLE project_journey
(
    tenant_id                      TEXT NOT NULL,
    project_id                     TEXT NOT NULL,
    opentelemetry_context          TEXT NOT NULL,
    start_datetime                 TEXT NOT NULL,
    previous_opentelemetry_context TEXT,
    PRIMARY KEY (tenant_id, project_id, opentelemetry_context)
);

CREATE TABLE host_journey
(
    tenant_id                      TEXT NOT NULL,
    opentelemetry_context          TEXT NOT NULL,
    start_datetime                 TEXT NOT NULL,
    previous_opentelemetry_context TEXT,
    PRIMARY KEY (tenant_id, opentelemetry_context)
);
