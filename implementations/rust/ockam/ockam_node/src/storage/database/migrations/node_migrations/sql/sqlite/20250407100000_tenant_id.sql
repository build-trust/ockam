-- Add a tenant_id column to all tables
ALTER TABLE aead_secret
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE authority_enrollment_token
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE authority_member
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE credential
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE identity
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE identity_attributes
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE identity_enrollment
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE incoming_service
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE kafka_config
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE named_identity
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE node
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE okta_config
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE project
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE purpose_key
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE resource
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE resource_policy
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE resource_type_policy
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE secure_channel
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE signing_secret
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE space
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE subscription
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE tcp_inlet
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE tcp_outlet_status
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE "user"
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE user_project
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE user_role
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE user_space
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE vault
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
ALTER TABLE x25519_secret
    ADD tenant_id TEXT NOT NULL DEFAULT 'tenant-id';
