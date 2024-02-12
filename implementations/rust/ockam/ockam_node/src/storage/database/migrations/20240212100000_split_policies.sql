-- Rename `policy` table
ALTER TABLE policy RENAME TO resource_policy;
ALTER TABLE resource_policy RENAME COLUMN resource TO resource_name;
DROP INDEX IF EXISTS policy_index;
CREATE UNIQUE INDEX resource_policy_index ON resource_policy (node_name, resource_name, action);

-- Create a new table for resource type policies
CREATE TABLE resource_type_policy
(
    resource_type   TEXT NOT NULL, -- resource type
    action          TEXT NOT NULL, -- action name
    expression      TEXT NOT NULL, -- encoded expression to evaluate
    node_name       TEXT NOT NULL  -- node name
);
CREATE UNIQUE INDEX resource_type_policy_index ON resource_type_policy (node_name, resource_type, action);

-- Create a new table for resource to resource type mapping
CREATE TABLE resource
(
    resource_name   TEXT NOT NULL, -- resource name
    resource_type   TEXT NOT NULL, -- resource type
    node_name       TEXT NOT NULL  -- node name
);
CREATE UNIQUE INDEX resource_index ON resource (node_name, resource_name, resource_type);

