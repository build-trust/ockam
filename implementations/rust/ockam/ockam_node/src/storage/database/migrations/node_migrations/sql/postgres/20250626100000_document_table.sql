CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS document
(
    id        TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    scope     TEXT,
    name      TEXT,
    text      TEXT
);

CREATE INDEX document_id_tenant_index ON document (id, tenant_id);

ALTER TABLE document
    ENABLE ROW LEVEL SECURITY;
CREATE POLICY document_policy ON document USING (tenant_id = current_user);

CREATE TABLE IF NOT EXISTS document_piece
(
    id        TEXT PRIMARY KEY,
    tenant_id TEXT   NOT NULL,
    scope     TEXT,
    name      TEXT,
    text      TEXT,
    embedding vector NOT NULL
);

CREATE INDEX document_piece_id_tenant_index ON document_piece (id, tenant_id);

ALTER TABLE document_piece
    ENABLE ROW LEVEL SECURITY;
CREATE POLICY document_pieces_policy ON document_piece USING (tenant_id = current_user);
