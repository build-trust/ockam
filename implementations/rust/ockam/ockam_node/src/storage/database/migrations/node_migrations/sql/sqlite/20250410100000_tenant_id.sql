-- identity
CREATE TABLE identity_new
(
    tenant_id      TEXT NOT NULL,
    identifier     TEXT NOT NULL,
    change_history TEXT NOT NULL,
    PRIMARY KEY (tenant_id, identifier)
);

INSERT INTO identity_new (tenant_id, identifier, change_history)
SELECT 'tenant-id-default' as tenant_id, identifier, change_history
FROM identity;
DROP TABLE identity;
ALTER TABLE identity_new
    RENAME TO identity;

-- named_identity
CREATE TABLE named_identity_new
(
    tenant_id  TEXT NOT NULL,
    identifier TEXT NOT NULL,
    name       TEXT NOT NULL,
    vault_name TEXT NOT NULL,
    is_default INTEGER DEFAULT 0,
    PRIMARY KEY (tenant_id, identifier)
);

INSERT INTO named_identity_new (tenant_id, identifier, name, vault_name, is_default)
SELECT 'tenant-id-default' as tenant_id, identifier, name, vault_name, is_default
FROM named_identity;
DROP TABLE named_identity;
ALTER TABLE named_identity_new
    RENAME TO named_identity;

CREATE UNIQUE INDEX named_identity_index ON named_identity (tenant_id, name);

-- identity_attributes
CREATE TABLE identity_attributes_new
(
    tenant_id   TEXT    NOT NULL,
    identifier  TEXT    NOT NULL,
    attributes  BLOB    NOT NULL,
    added       INTEGER NOT NULL,
    expires     INTEGER,
    attested_by TEXT,
    node_name   TEXT    NOT NULL,
    PRIMARY KEY (tenant_id, identifier, node_name)
);

INSERT INTO identity_attributes_new (tenant_id, identifier, attributes, added, expires, attested_by, node_name)
SELECT 'tenant-id-default' as tenant_id, identifier, attributes, added, expires, attested_by, node_name
FROM identity_attributes;
DROP TABLE identity_attributes;
ALTER TABLE identity_attributes_new
    RENAME TO identity_attributes;

CREATE UNIQUE INDEX identity_attributes_index ON identity_attributes (tenant_id, identifier, node_name);
CREATE INDEX identity_attributes_identifier_attested_by_node_name_index ON identity_attributes (tenant_id, identifier, attested_by, node_name);
CREATE INDEX identity_attributes_expires_node_name_index ON identity_attributes (tenant_id, expires, node_name);
CREATE INDEX identity_identifier_index ON identity_attributes (tenant_id, identifier);
CREATE INDEX identity_node_name_index ON identity_attributes (tenant_id, node_name);

-- credential
CREATE TABLE credential_new
(
    tenant_id          TEXT NOT NULL,
    subject_identifier TEXT NOT NULL,
    issuer_identifier  TEXT NOT NULL,
    scope              TEXT NOT NULL,
    credential         BLOB NOT NULL,
    expires_at         INTEGER,
    node_name          TEXT NOT NULL,
    PRIMARY KEY (tenant_id, subject_identifier, issuer_identifier, scope)
);

INSERT INTO credential_new (tenant_id, subject_identifier, issuer_identifier, scope, credential, expires_at, node_name)
SELECT 'tenant-id-default' as tenant_id, subject_identifier, issuer_identifier, scope, credential, expires_at, node_name
FROM credential;
DROP TABLE credential;
ALTER TABLE credential_new
    RENAME TO credential;

CREATE UNIQUE INDEX credential_issuer_subject_scope_index ON credential (tenant_id, issuer_identifier, subject_identifier, scope);
CREATE INDEX credential_issuer_subject_index ON credential (tenant_id, issuer_identifier, subject_identifier);

-- purpose_key
CREATE TABLE purpose_key_new
(
    tenant_id               TEXT NOT NULL,
    identifier              TEXT NOT NULL,
    purpose                 TEXT NOT NULL,
    purpose_key_attestation BLOB NOT NULL,
    PRIMARY KEY (tenant_id, identifier, purpose)
);

INSERT INTO purpose_key_new (tenant_id, identifier, purpose, purpose_key_attestation)
SELECT 'tenant-id-default' as tenant_id, identifier, purpose, purpose_key_attestation
FROM purpose_key;
DROP TABLE purpose_key;
ALTER TABLE purpose_key_new
    RENAME TO purpose_key;

CREATE UNIQUE INDEX purpose_key_index ON purpose_key (tenant_id, identifier, purpose);

-- vault
CREATE TABLE vault_new
(
    tenant_id  TEXT NOT NULL,
    name       TEXT NOT NULL,
    path       TEXT NULL,
    is_default INTEGER,
    is_kms     INTEGER,
    PRIMARY KEY (tenant_id, name)
);

INSERT INTO vault_new (tenant_id, name, path, is_default, is_kms)
SELECT 'tenant-id-default' as tenant_id, name, path, is_default, is_kms
FROM vault;
DROP TABLE vault;
ALTER TABLE vault_new
    RENAME TO vault;

-- signing_secret
CREATE TABLE signing_secret_new
(
    tenant_id   TEXT NOT NULL,
    handle      BLOB NOT NULL,
    secret_type TEXT NOT NULL,
    secret      BLOB NOT NULL,
    PRIMARY KEY (tenant_id, handle)
);

INSERT INTO signing_secret_new (tenant_id, handle, secret_type, secret)
SELECT 'tenant-id-default' as tenant_id, handle, secret_type, secret
FROM signing_secret;
DROP TABLE signing_secret;
ALTER TABLE signing_secret_new
    RENAME TO signing_secret;

-- x25519_secret
CREATE TABLE x25519_secret_new
(
    tenant_id TEXT NOT NULL,
    handle    BLOB NOT NULL,
    secret    BLOB NOT NULL,
    PRIMARY KEY (tenant_id, handle)
);

INSERT INTO x25519_secret_new (tenant_id, handle, secret)
SELECT 'tenant-id-default' as tenant_id, handle, secret
FROM x25519_secret;
DROP TABLE x25519_secret;
ALTER TABLE x25519_secret_new
    RENAME TO x25519_secret;

-- authority_member
CREATE TABLE authority_member_new
(
    tenant_id      TEXT    NOT NULL,
    identifier     TEXT    NOT NULL,
    added_by       TEXT    NOT NULL,
    added_at       INTEGER NOT NULL,
    is_pre_trusted INTEGER NOT NULL,
    attributes     BLOB,
    authority_id   TEXT    NOT NULL,
    PRIMARY KEY (tenant_id, identifier)
);

INSERT INTO authority_member_new (tenant_id, identifier, added_by, added_at, is_pre_trusted, attributes, authority_id)
SELECT 'tenant-id-default' as tenant_id, identifier, added_by, added_at, is_pre_trusted, attributes, authority_id
FROM authority_member;
DROP TABLE authority_member;
ALTER TABLE authority_member_new
    RENAME TO authority_member;

CREATE INDEX authority_member_is_pre_trusted_index ON authority_member (tenant_id, is_pre_trusted);

-- authority_enrollment_token
CREATE TABLE authority_enrollment_token_new
(
    tenant_id     TEXT    NOT NULL,
    one_time_code TEXT    NOT NULL,
    issued_by     TEXT    NOT NULL,
    created_at    INTEGER NOT NULL,
    expires_at    INTEGER NOT NULL,
    ttl_count     INTEGER NOT NULL,
    attributes    BLOB,
    reference     TEXT,
    PRIMARY KEY (tenant_id, one_time_code)
);

INSERT INTO authority_enrollment_token_new (tenant_id, one_time_code, issued_by, created_at, expires_at, ttl_count,
                                            attributes, reference)
SELECT 'tenant-id-default' as tenant_id,
       one_time_code,
       issued_by,
       created_at,
       expires_at,
       ttl_count,
       attributes,
       reference
FROM authority_enrollment_token;
DROP TABLE authority_enrollment_token;
ALTER TABLE authority_enrollment_token_new
    RENAME TO authority_enrollment_token;

CREATE INDEX authority_enrollment_token_expires_at_index ON authority_enrollment_token (tenant_id, expires_at);

-- resource_policy
CREATE TABLE resource_policy_new
(
    tenant_id     TEXT NOT NULL,
    resource_name TEXT NOT NULL,
    action        TEXT NOT NULL,
    expression    TEXT NOT NULL,
    node_name     TEXT NOT NULL,
    PRIMARY KEY (tenant_id, node_name, resource_name, action)
);

INSERT INTO resource_policy_new (tenant_id, resource_name, action, expression, node_name)
SELECT 'tenant-id-default' as tenant_id, resource_name, action, expression, node_name
FROM resource_policy;
DROP TABLE resource_policy;
ALTER TABLE resource_policy_new
    RENAME TO resource_policy;

-- resource_type_policy
CREATE TABLE resource_type_policy_new
(
    tenant_id     TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    action        TEXT NOT NULL,
    expression    TEXT NOT NULL,
    node_name     TEXT NOT NULL,
    PRIMARY KEY (tenant_id, node_name, resource_type, action)
);

INSERT INTO resource_type_policy_new (tenant_id, resource_type, action, expression, node_name)
SELECT 'tenant-id-default' as tenant_id, resource_type, action, expression, node_name
FROM resource_type_policy;
DROP TABLE resource_type_policy;
ALTER TABLE resource_type_policy_new
    RENAME TO resource_type_policy;

-- resource
CREATE TABLE resource_new
(
    tenant_id     TEXT NOT NULL,
    resource_name TEXT NOT NULL,
    resource_type TEXT,
    node_name     TEXT NOT NULL,
    PRIMARY KEY (tenant_id, node_name, resource_name)
);

INSERT INTO resource_new (tenant_id, resource_name, resource_type, node_name)
SELECT 'tenant-id-default' as tenant_id, resource_name, resource_type, node_name
FROM resource;
DROP TABLE resource;
ALTER TABLE resource_new
    RENAME TO resource;

-- tcp_outlet_status
CREATE TABLE tcp_outlet_status_new
(
    tenant_id   TEXT NOT NULL,
    node_name   TEXT NOT NULL,
    socket_addr TEXT NOT NULL,
    worker_addr TEXT NOT NULL,
    payload     TEXT,
    privileged  INTEGER DEFAULT 0,
    PRIMARY KEY (tenant_id, node_name, socket_addr)
);

INSERT INTO tcp_outlet_status_new (tenant_id, node_name, socket_addr, worker_addr, payload, privileged)
SELECT 'tenant-id-default' as tenant_id, node_name, socket_addr, worker_addr, payload, privileged
FROM tcp_outlet_status;
DROP TABLE tcp_outlet_status;
ALTER TABLE tcp_outlet_status_new
    RENAME TO tcp_outlet_status;

-- tcp_inlet
CREATE TABLE tcp_inlet_new
(
    tenant_id   TEXT NOT NULL,
    node_name   TEXT NOT NULL,
    bind_addr   TEXT NOT NULL,
    outlet_addr TEXT NOT NULL,
    alias       TEXT NOT NULL,
    privileged  INTEGER DEFAULT 0,
    PRIMARY KEY (tenant_id, node_name, bind_addr)
);

INSERT INTO tcp_inlet_new (tenant_id, node_name, bind_addr, outlet_addr, alias, privileged)
SELECT 'tenant-id-default' as tenant_id, node_name, bind_addr, outlet_addr, alias, privileged
FROM tcp_inlet;
DROP TABLE tcp_inlet;
ALTER TABLE tcp_inlet_new
    RENAME TO tcp_inlet;

-- node
CREATE TABLE node_new
(
    tenant_id            TEXT    NOT NULL,
    name                 TEXT    NOT NULL,
    identifier           TEXT    NOT NULL,
    verbosity            INTEGER NOT NULL,
    is_default           INTEGER NOT NULL,
    is_authority         INTEGER NOT NULL,
    tcp_listener_address TEXT,
    pid                  INTEGER,
    http_server_address  TEXT,
    PRIMARY KEY (tenant_id, name)
);

INSERT INTO node_new (tenant_id, name, identifier, verbosity, is_default, is_authority, tcp_listener_address, pid,
                      http_server_address)
SELECT 'tenant-id-default' as tenant_id,
       name,
       identifier,
       verbosity,
       is_default,
       is_authority,
       tcp_listener_address,
       pid,
       http_server_address
FROM node;
DROP TABLE node;
ALTER TABLE node_new
    RENAME TO node;

-- secure_channel
CREATE TABLE secure_channel_new
(
    tenant_id                TEXT NOT NULL,
    role                     TEXT NOT NULL,
    my_identifier            TEXT NOT NULL,
    their_identifier         TEXT NOT NULL,
    decryptor_remote_address TEXT NOT NULL,
    decryptor_api_address    TEXT NOT NULL,
    decryption_key_handle    BLOB NOT NULL,
    PRIMARY KEY (tenant_id, decryptor_remote_address)
);

INSERT INTO secure_channel_new (tenant_id, role, my_identifier, their_identifier, decryptor_remote_address,
                                decryptor_api_address, decryption_key_handle)
SELECT 'tenant-id-default' as tenant_id,
       role,
       my_identifier,
       their_identifier,
       decryptor_remote_address,
       decryptor_api_address,
       decryption_key_handle
FROM secure_channel;
DROP TABLE secure_channel;
ALTER TABLE secure_channel_new
    RENAME TO secure_channel;

-- aead_secret
CREATE TABLE aead_secret_new
(
    tenant_id TEXT NOT NULL,
    handle    BLOB NOT NULL,
    type      TEXT NOT NULL,
    secret    BLOB NOT NULL,
    PRIMARY KEY (tenant_id, handle)
);

INSERT INTO aead_secret_new (tenant_id, handle, type, secret)
SELECT 'tenant-id-default' as tenant_id, handle, type, secret
FROM aead_secret;
DROP TABLE aead_secret;
ALTER TABLE aead_secret_new
    RENAME TO aead_secret;

-- project
CREATE TABLE project_new
(
    tenant_id                TEXT    NOT NULL,
    project_id               TEXT    NOT NULL,
    project_name             TEXT    NOT NULL,
    is_default               INTEGER NOT NULL,
    space_id                 TEXT    NOT NULL,
    space_name               TEXT    NOT NULL,
    project_identifier       TEXT,
    project_change_history   TEXT,
    access_route             TEXT    NOT NULL,
    authority_change_history TEXT,
    authority_access_route   TEXT,
    version                  TEXT,
    running                  INTEGER,
    operation_id             TEXT,
    PRIMARY KEY (tenant_id, project_id)
);

INSERT INTO project_new (tenant_id, project_id, project_name, is_default, space_id, space_name, project_identifier,
                         project_change_history, access_route, authority_change_history, authority_access_route,
                         version, running, operation_id)
SELECT 'tenant-id-default' as tenant_id,
       project_id,
       project_name,
       is_default,
       space_id,
       space_name,
       project_identifier,
       project_change_history,
       access_route,
       authority_change_history,
       authority_access_route,
       version,
       running,
       operation_id
FROM project;
DROP TABLE project;
ALTER TABLE project_new
    RENAME TO project;

-- user_project
CREATE TABLE user_project_new
(
    tenant_id  TEXT NOT NULL,
    user_email TEXT NOT NULL,
    project_id TEXT NOT NULL,
    PRIMARY KEY (tenant_id, user_email, project_id)
);

INSERT INTO user_project_new (tenant_id, user_email, project_id)
SELECT 'tenant-id-default' as tenant_id, user_email, project_id
FROM user_project;
DROP TABLE user_project;
ALTER TABLE user_project_new
    RENAME TO user_project;

-- user_rol
CREATE TABLE user_role_new
(
    tenant_id  TEXT    NOT NULL,
    user_id    INTEGER NOT NULL,
    project_id TEXT    NOT NULL,
    user_email TEXT    NOT NULL,
    role       TEXT    NOT NULL,
    scope      TEXT    NOT NULL,
    PRIMARY KEY (tenant_id, user_id, project_id)
);

INSERT INTO user_role_new (tenant_id, user_id, project_id, user_email, role, scope)
SELECT 'tenant-id-default' as tenant_id, user_id, project_id, user_email, role, scope
FROM user_role;
DROP TABLE user_role;
ALTER TABLE user_role_new
    RENAME TO user_role;

-- space
CREATE TABLE space_new
(
    tenant_id  TEXT    NOT NULL,
    space_id   TEXT    NOT NULL,
    space_name TEXT    NOT NULL,
    is_default INTEGER NOT NULL,
    PRIMARY KEY (tenant_id, space_id)
);

INSERT INTO space_new (tenant_id, space_id, space_name, is_default)
SELECT 'tenant-id-default' as tenant_id, space_id, space_name, is_default
FROM space;
DROP TABLE space;
ALTER TABLE space_new
    RENAME TO space;

-- user_space
CREATE TABLE user_space_new
(
    tenant_id  TEXT NOT NULL,
    user_email TEXT NOT NULL,
    space_id   TEXT NOT NULL,
    PRIMARY KEY (tenant_id, user_email, space_id)
);

INSERT INTO user_space_new (tenant_id, user_email, space_id)
SELECT 'tenant-id-default' as tenant_id, user_email, space_id
FROM user_space;
DROP TABLE user_space;
ALTER TABLE user_space_new
    RENAME TO user_space;

-- subscription
CREATE TABLE subscription_new
(
    tenant_id     TEXT    NOT NULL,
    space_id      TEXT    NOT NULL,
    name          TEXT    NOT NULL,
    is_free_trial INTEGER NOT NULL,
    marketplace   TEXT,
    start_date    INTEGER,
    end_date      INTEGER,
    PRIMARY KEY (tenant_id, space_id)
);

INSERT INTO subscription_new (tenant_id, space_id, name, is_free_trial, marketplace, start_date, end_date)
SELECT 'tenant-id-default' as tenant_id, space_id, name, is_free_trial, marketplace, start_date, end_date
FROM subscription;
DROP TABLE subscription;
ALTER TABLE subscription_new
    RENAME TO subscription;

-- user
CREATE TABLE "user_new"
(
    tenant_id      TEXT    NOT NULL,
    email          TEXT    NOT NULL,
    sub            TEXT    NOT NULL,
    nickname       TEXT    NOT NULL,
    name           TEXT    NOT NULL,
    picture        TEXT    NOT NULL,
    updated_at     TEXT    NOT NULL,
    email_verified INTEGER NOT NULL,
    is_default     INTEGER NOT NULL,
    PRIMARY KEY (tenant_id, email)
);
INSERT INTO "user_new" (tenant_id, email, sub, nickname, name, picture, updated_at, email_verified, is_default)
SELECT 'tenant-id-default' as tenant_id,
       email,
       sub,
       nickname,
       name,
       picture,
       updated_at,
       email_verified,
       is_default
FROM "user";
DROP TABLE "user";
ALTER TABLE "user_new"
    RENAME TO "user";

-- identity
CREATE TABLE identity_enrollment_new
(
    tenant_id   TEXT    NOT NULL,
    identifier  TEXT    NOT NULL,
    enrolled_at INTEGER NOT NULL,
    email       TEXT,
    PRIMARY KEY (tenant_id, identifier)
);

INSERT INTO identity_enrollment_new (tenant_id, identifier, enrolled_at, email)
SELECT 'tenant-id-default' as tenant_id, identifier, enrolled_at, email
FROM identity_enrollment;
DROP TABLE identity_enrollment;
ALTER TABLE identity_enrollment_new
    RENAME TO identity_enrollment;

-- okta_config
CREATE TABLE okta_config_new
(
    tenant_id       TEXT NOT NULL,
    project_id      TEXT NOT NULL,
    tenant_base_url TEXT NOT NULL,
    client_id       TEXT NOT NULL,
    certificate     TEXT NOT NULL,
    attributes      TEXT,
    PRIMARY KEY (tenant_id, project_id)
);

INSERT INTO okta_config_new (tenant_id, project_id, tenant_base_url, client_id, certificate, attributes)
SELECT 'tenant-id-default' as tenant_id, project_id, tenant_base_url, client_id, certificate, attributes
FROM okta_config;
DROP TABLE okta_config;
ALTER TABLE okta_config_new
    RENAME TO okta_config;

-- kafka_config
CREATE TABLE kafka_config_new
(
    tenant_id        TEXT NOT NULL,
    project_id       TEXT NOT NULL,
    bootstrap_server TEXT NOT NULL
);

INSERT INTO kafka_config_new (tenant_id, project_id, bootstrap_server)
SELECT 'tenant-id-default' as tenant_id, project_id, bootstrap_server
FROM kafka_config;
DROP TABLE kafka_config;
ALTER TABLE kafka_config_new
    RENAME TO kafka_config;

-- incoming_service
CREATE TABLE incoming_service_new
(
    tenant_id     TEXT    NOT NULL,
    invitation_id TEXT    NOT NULL,
    enabled       INTEGER NOT NULL,
    name          TEXT    NULL,
    PRIMARY KEY (tenant_id, invitation_id)
);

INSERT INTO incoming_service_new (tenant_id, invitation_id, enabled, name)
SELECT 'tenant-id-default', invitation_id, enabled, name
FROM incoming_service;
DROP TABLE incoming_service;
ALTER TABLE incoming_service_new
    RENAME TO incoming_service;
