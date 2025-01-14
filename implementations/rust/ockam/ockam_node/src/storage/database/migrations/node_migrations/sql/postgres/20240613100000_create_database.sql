--------------
-- MIGRATIONS
--------------

-- Create a table to support rust migrations
CREATE TABLE IF NOT EXISTS _rust_migrations
(
    name   TEXT      NOT NULL,
    run_on TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS name_index ON _rust_migrations (name);

--------------
-- IDENTITIES
--------------

-- This table stores identities with
--  - the identity identifier (as a hex-encoded string)
--  - the encoded history of all the key rotations for this identity
CREATE TABLE identity
(
    identifier     TEXT NOT NULL UNIQUE,
    change_history TEXT NOT NULL
);

-- This table some local metadata about identities
CREATE TABLE named_identity
(
    identifier TEXT NOT NULL UNIQUE, -- Identity identifier
    name       TEXT UNIQUE,          -- user-specified name
    vault_name TEXT NOT NULL,        -- name of the vault used to store the identity keys
    is_default BOOLEAN DEFAULT FALSE -- boolean indicating if this identity is the default one
);


-- This table lists attributes associated to a given identity
CREATE TABLE identity_attributes
(
    identifier  TEXT PRIMARY KEY, -- identity possessing those attributes
    attributes  BYTEA   NOT NULL, -- serialized list of attribute names and values for the identity
    added       INTEGER NOT NULL, -- UNIX timestamp in seconds: when those attributes were inserted in the database
    expires     INTEGER,          -- optional UNIX timestamp in seconds: when those attributes expire
    attested_by TEXT,             -- optional identifier which attested of these attributes
    node_name   TEXT    NOT NULL  -- node name to isolate attributes that each node knows
);

CREATE UNIQUE INDEX identity_attributes_index ON identity_attributes (identifier, node_name);

CREATE INDEX identity_attributes_identifier_attested_by_node_name_index ON identity_attributes (identifier, attested_by, node_name);

CREATE INDEX identity_attributes_expires_node_name_index ON identity_attributes (expires, node_name);

CREATE INDEX identity_identifier_index ON identity_attributes (identifier);

CREATE INDEX identity_node_name_index ON identity_attributes (node_name);

-- This table stores credentials as received by the application
CREATE TABLE credential
(
    subject_identifier TEXT  NOT NULL,
    issuer_identifier  TEXT  NOT NULL,
    scope              TEXT  NOT NULL,
    credential         BYTEA NOT NULL,
    expires_at         INTEGER,
    node_name          TEXT  NOT NULL -- node name to isolate credential that each node has
);

CREATE UNIQUE INDEX credential_issuer_subject_scope_index ON credential (issuer_identifier, subject_identifier, scope);
CREATE UNIQUE INDEX credential_issuer_subject_index ON credential (issuer_identifier, subject_identifier);

-- This table stores purpose keys that have been created by a given identity
CREATE TABLE purpose_key
(
    identifier              TEXT  NOT NULL, -- Identity identifier
    purpose                 TEXT  NOT NULL, -- Purpose of the key: SecureChannels, or Credentials
    purpose_key_attestation BYTEA NOT NULL  -- Encoded attestation: attestation data and attestation signature
);

CREATE UNIQUE INDEX purpose_key_index ON purpose_key (identifier, purpose);

----------
-- VAULTS
----------

-- This table stores vault metadata when several vaults have been created locally
CREATE TABLE vault
(
    name       TEXT PRIMARY KEY, -- User-specified name for a vault
    path       TEXT NULL,        -- If the path is specified, then the secrets are stored in a SQLite file. Otherwise secrets are stored in the *-secrets tables below.
    is_default BOOLEAN,          -- boolean indicating if this vault is the default one (0 means true)
    is_kms     BOOLEAN           -- boolean indicating if this vault is a KMS one (0 means true). In that case only key handles are stored in the database
);

-- This table stores secrets for signing data
CREATE TABLE signing_secret
(
    handle      BYTEA PRIMARY KEY, -- Secret handle
    secret_type TEXT  NOT NULL,    -- Secret type (EdDSACurve25519 or ECDSASHA256CurveP256)
    secret      BYTEA NOT NULL     -- Secret binary
);

-- This table stores secrets for encrypting / decrypting data
CREATE TABLE x25519_secret
(
    handle BYTEA PRIMARY KEY, -- Secret handle
    secret BYTEA NOT NULL     -- Secret binary
);

-------------
-- AUTHORITY
-------------

CREATE TABLE authority_member
(
    identifier     TEXT    NOT NULL UNIQUE,
    added_by       TEXT    NOT NULL,
    added_at       INTEGER NOT NULL,
    is_pre_trusted BOOLEAN NOT NULL,
    attributes     BYTEA,
    authority_id   TEXT    NOT NULL
);

CREATE UNIQUE INDEX authority_member_identifier_index ON authority_member (identifier);
CREATE INDEX authority_member_is_pre_trusted_index ON authority_member (is_pre_trusted);

-- Reference is a random string that uniquely identifies an enrollment token. However, unlike the one_time_code,
-- it's not sensitive so can be logged and used to track a lifecycle of a specific enrollment token.
CREATE TABLE authority_enrollment_token
(
    one_time_code TEXT    NOT NULL UNIQUE,
    issued_by     TEXT    NOT NULL,
    created_at    INTEGER NOT NULL,
    expires_at    INTEGER NOT NULL,
    ttl_count     INTEGER NOT NULL,
    attributes    BYTEA,
    reference     TEXT
);

CREATE UNIQUE INDEX authority_enrollment_token_one_time_code_index ON authority_enrollment_token (one_time_code);
CREATE INDEX authority_enrollment_token_expires_at_index ON authority_enrollment_token (expires_at);

------------
-- SERVICES
------------

-- This table stores policies. A policy is an expression which
-- can be evaluated against an environment (a list of name/value pairs)
-- to assess if a given action can be performed on a given resource
CREATE TABLE resource_policy
(
    resource_name TEXT NOT NULL, -- resource name
    action        TEXT NOT NULL, -- action name
    expression    TEXT NOT NULL, -- encoded expression to evaluate
    node_name     TEXT NOT NULL  -- node name
);

CREATE UNIQUE INDEX resource_policy_index ON resource_policy (node_name, resource_name, action);

-- Create a new table for resource type policies
CREATE TABLE resource_type_policy
(
    resource_type TEXT NOT NULL, -- resource type
    action        TEXT NOT NULL, -- action name
    expression    TEXT NOT NULL, -- encoded expression to evaluate
    node_name     TEXT NOT NULL  -- node name
);
CREATE UNIQUE INDEX resource_type_policy_index ON resource_type_policy (node_name, resource_type, action);

-- Create a new table for resource to resource type mapping
CREATE TABLE resource
(
    resource_name TEXT NOT NULL, -- resource name
    resource_type TEXT NOT NULL, -- resource type
    node_name     TEXT NOT NULL  -- node name
);
CREATE UNIQUE INDEX resource_index ON resource (node_name, resource_name, resource_type);

-- This table stores the current state of a TCP outlet
CREATE TABLE tcp_outlet_status
(
    node_name   TEXT NOT NULL,        -- Node where that tcp outlet has been created
    socket_addr TEXT NOT NULL,        -- Socket address that the outlet connects to
    worker_addr TEXT NOT NULL,        -- Worker address for the outlet itself
    payload     TEXT,                 -- Optional status payload
    privileged  BOOLEAN DEFAULT FALSE -- boolean indicating if the outlet is operating in privileged mode
);

-- This table stores the current state of a TCP inlet
CREATE TABLE tcp_inlet
(
    node_name   TEXT NOT NULL,        -- Node where that tcp inlet has been created
    bind_addr   TEXT NOT NULL,        -- Input address to connect to
    outlet_addr TEXT NOT NULL,        -- MultiAddress to the outlet
    alias       TEXT NOT NULL,        -- Alias for that inlet
    privileged  BOOLEAN DEFAULT FALSE -- boolean indicating if the inlet is operating in privileged mode
);

---------
-- NODES
---------

-- This table stores information about local nodes
CREATE TABLE node
(
    name                 TEXT PRIMARY KEY, -- Node name
    identifier           TEXT    NOT NULL, -- Identifier of the default identity associated to the node
    verbosity            INTEGER NOT NULL, -- Verbosity level used for logging
    is_default           BOOLEAN NOT NULL, -- boolean indicating if this node is the default one (0 means true)
    is_authority         BOOLEAN NOT NULL, -- boolean indicating if this node is an authority node (0 means true). This boolean is used to be able to show an authority node as UP even if its TCP listener cannot be accessed.
    tcp_listener_address TEXT,             -- Socket address for the node default TCP Listener (can be NULL if the node has not been started)
    pid                  INTEGER,          -- Current process id of the node if it has been started
    http_server_address  TEXT              -- Address of the server supporting the HTTP status endpoint for the node
);

-------------------
-- SECURE CHANNELS
-------------------

-- This table stores secure channels in order to restore them on a restart
CREATE TABLE secure_channel
(
    role                     TEXT  NOT NULL,
    my_identifier            TEXT  NOT NULL,
    their_identifier         TEXT  NOT NULL,
    decryptor_remote_address TEXT PRIMARY KEY,
    decryptor_api_address    TEXT  NOT NULL,
    decryption_key_handle    BYTEA NOT NULL
    -- TODO: Add date?
);

CREATE UNIQUE INDEX secure_channel_decryptor_api_address_index ON secure_channel (decryptor_remote_address);

-- This table stores aead secrets
CREATE TABLE aead_secret
(
    handle BYTEA PRIMARY KEY, -- Secret handle
    type   TEXT  NOT NULL,    -- Secret type
    secret BYTEA NOT NULL     -- Secret binary
);
