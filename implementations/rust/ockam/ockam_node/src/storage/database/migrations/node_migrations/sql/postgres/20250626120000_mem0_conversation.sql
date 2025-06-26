CREATE TABLE IF NOT EXISTS mem0_conversation
(
    id        UUID PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    vector    VECTOR,
    payload   JSONB
);

CREATE INDEX mem0_conversation_id_tenant_index ON mem0_conversation (id, tenant_id);

ALTER TABLE mem0_conversation
    ENABLE ROW LEVEL SECURITY;
CREATE POLICY mem0_conversation_policy ON mem0_conversation USING (tenant_id = current_user);
