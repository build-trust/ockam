ALTER TABLE policy RENAME TO policy_old;

-- Add a new column to the table "node" to isolate policies by node
-- The rust migration will handle the data migration between the old and new table
CREATE TABLE policy
(
    node       TEXT NOT NULL, -- node name
    resource   TEXT NOT NULL, -- resource name
    action     TEXT NOT NULL, -- action name
    expression BLOB NOT NULL  -- expression to evaluate
);

DROP INDEX IF EXISTS policy_index;
CREATE UNIQUE INDEX policy_index ON policy (node, resource, action);
