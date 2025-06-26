CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS conversation
(
    tenant_id    TEXT NOT NULL,
    scope        TEXT,
    conversation TEXT,
    message      TEXT
);
CREATE INDEX conversation_tenant_index ON conversation (tenant_id);
CREATE INDEX conversation_tenant_scope_conversation_index ON conversation (tenant_id, scope, conversation);

ALTER TABLE conversation
    ENABLE ROW LEVEL SECURITY;

CREATE POLICY conversation_policy ON conversation USING (tenant_id = current_user);
