DROP TABLE trust_context;

CREATE INDEX identity_attributes_attested_by_index ON identity_attributes (identifier, attested_by);

ALTER TABLE policy RENAME TO policy_old;

-- Add a new column to the table "node" to isolate policies by node
-- Use string representation for the expression
-- The rust migration will handle the data migration between the old and new table
CREATE TABLE policy
(
    resource   TEXT NOT NULL, -- resource name
    action     TEXT NOT NULL, -- action name
    expression TEXT NOT NULL, -- encoded expression to evaluate
    node_name  TEXT NOT NULL  -- node name
);

DROP INDEX IF EXISTS policy_index;
CREATE UNIQUE INDEX policy_index ON policy (node_name, resource, action);
