-- project_journey
CREATE TABLE project_journey_new
(
    tenant_id                      TEXT NOT NULL,
    project_id                     TEXT NOT NULL,
    opentelemetry_context          TEXT NOT NULL,
    start_datetime                 TEXT NOT NULL,
    previous_opentelemetry_context TEXT,
    PRIMARY KEY (tenant_id, project_id, opentelemetry_context)
);

INSERT INTO project_journey_new (tenant_id, project_id, opentelemetry_context, start_datetime,
                                 previous_opentelemetry_context)
SELECT 'no-tenant-id', project_id, opentelemetry_context, start_datetime, previous_opentelemetry_context
FROM project_journey;
DROP TABLE project_journey;
ALTER TABLE project_journey_new
    RENAME TO project_journey;

-- host_journey
CREATE TABLE host_journey_new
(
    tenant_id                      TEXT NOT NULL,
    opentelemetry_context          TEXT NOT NULL,
    start_datetime                 TEXT NOT NULL,
    previous_opentelemetry_context TEXT,
    PRIMARY KEY (tenant_id, opentelemetry_context)
);

INSERT INTO host_journey_new (tenant_id, opentelemetry_context, start_datetime, previous_opentelemetry_context)
SELECT 'no-tenant-id', opentelemetry_context, start_datetime, previous_opentelemetry_context
FROM host_journey;
DROP TABLE host_journey;
ALTER TABLE host_journey_new
    RENAME TO host_journey;
