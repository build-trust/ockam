DROP TABLE credential;

CREATE TABLE credential
(
    subject_identifier TEXT NOT NULL,
    issuer_identifier  TEXT NOT NULL,
    scope              TEXT NOT NULL,
    credential         TEXT NOT NULL,
    expires_at         INTEGER,
    node_name          TEXT NOT NULL -- node name to isolate credential that each node has
);

CREATE UNIQUE INDEX credential_issuer_subject_scope_index ON credential (issuer_identifier, subject_identifier, scope);

-- Replace index
DROP INDEX identity_attributes_attested_by_index;
CREATE INDEX identity_attributes_identifier_attested_by_node_name_index ON identity_attributes (identifier, attested_by, node_name);

CREATE INDEX identity_attributes_expires_node_name_index ON identity_attributes (expires, node_name);

CREATE INDEX identity_identifier_index ON identity_attributes (identifier);

CREATE INDEX identity_node_name_index ON identity_attributes (node_name);
