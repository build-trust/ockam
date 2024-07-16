-- We add a column to create several journeys
-- where each new journey has a greater start time and uses the previous_opentelemetry_context to point to the
-- previous journey

-- The migration for the project_journey table has to be done in several steps in order to remove the PRIMARY KEY
-- constraint on project_id
CREATE TABLE project_journey_copy AS SELECT * FROM project_journey;
CREATE TABLE project_journey_new (
  project_id            TEXT NOT NULL,
  opentelemetry_context TEXT NOT NULL UNIQUE,
  start_datetime        TEXT NOT NULL,
  previous_opentelemetry_context TEXT
);

INSERT INTO project_journey_new SELECT project_id, opentelemetry_context, start_datetime, NULL FROM project_journey_copy;
DROP TABLE project_journey;
DROP TABLE project_journey_copy;
ALTER TABLE project_journey_new RENAME TO project_journey;

ALTER TABLE host_journey ADD COLUMN previous_opentelemetry_context TEXT NULL;
