ALTER TABLE resource RENAME TO resource_old;
DROP INDEX IF EXISTS resource_index;

CREATE TABLE resource
(
    resource_name   TEXT NOT NULL, -- resource name
    resource_type   TEXT,          -- resource type
    node_name       TEXT NOT NULL  -- node name
);
CREATE UNIQUE INDEX resource_index ON resource (node_name, resource_name);

INSERT INTO resource (resource_name, resource_type, node_name)
SELECT resource_name, resource_type, node_name FROM resource_old;

DROP TABLE resource_old;
