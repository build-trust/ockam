CREATE TABLE project_journey (
    project_id            TEXT PRIMARY KEY,
    opentelemetry_context TEXT NOT NULL UNIQUE,
    start_datetime        TEXT NOT NULL
);

CREATE TABLE host_journey (
    opentelemetry_context TEXT NOT NULL UNIQUE,
    start_datetime        TEXT NOT NULL
);
