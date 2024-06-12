DROP TABLE credential;

CREATE TABLE credential
(
    subject_identifier TEXT NOT NULL,
    issuer_identifier  TEXT NOT NULL,
    credential         TEXT NOT NULL,
    expires_at         INTEGER,
    node_name          TEXT NOT NULL     -- node name to isolate credential that each node has
);

CREATE UNIQUE INDEX credential_issuer_subject_index ON credential(issuer_identifier, subject_identifier);
