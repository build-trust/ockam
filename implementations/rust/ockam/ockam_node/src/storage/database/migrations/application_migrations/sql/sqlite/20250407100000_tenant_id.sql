-- Add a tenant_id column to all tables
ALTER TABLE project_journey
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';

ALTER TABLE host_journey
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
