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
    is_default INTEGER DEFAULT 0     -- boolean indicating if this identity is the default one (0 means true)
);

-- This table stores the time when a given identity was enrolled
-- In the current project
CREATE TABLE identity_enrollment
(
    identifier  TEXT    NOT NULL UNIQUE, -- Identifier of the identity
    enrolled_at INTEGER NOT NULL         -- UNIX timestamp in seconds
);

-- This table lists attributes associated to a given identity
CREATE TABLE identity_attributes
(
    identifier  TEXT PRIMARY KEY, -- identity possessing those attributes
    attributes  BLOB    NOT NULL, -- serialized list of attribute names and values for the identity
    added       INTEGER NOT NULL, -- UNIX timestamp in seconds: when those attributes were inserted in the database
    expires     INTEGER,          -- optional UNIX timestamp in seconds: when those attributes expire
    attested_by TEXT              -- optional identifier which attested of these attributes
);

-- This table stores purpose keys that have been created by a given identity
CREATE TABLE purpose_key
(
    identifier              TEXT NOT NULL, -- Identity identifier
    purpose                 TEXT NOT NULL, -- Purpose of the key: SecureChannels, or Credentials
    purpose_key_attestation BLOB NOT NULL  -- Encoded attestation: attestation data and attestation signature
);

CREATE UNIQUE INDEX purpose_key_index ON purpose_key (identifier, purpose);

----------
-- VAULTS
----------

-- This table stores vault metadata when several vaults have been created locally
CREATE TABLE vault
(
    name       TEXT PRIMARY KEY, -- User-specified name for a vault
    path       TEXT NOT NULL,    -- Path where the vault is saved, This path can the current database path. In that case the vault data is stored in the *-secrets table below
    is_default INTEGER,          -- boolean indicating if this vault is the default one (0 means true)
    is_kms     INTEGER           -- boolean indicating if this vault is a KMS one (0 means true). In that case only key handles are stored in the database
);

-- This table stores secrets for signing data
CREATE TABLE signing_secret
(
    handle      BLOB PRIMARY KEY, -- Secret handle
    secret_type TEXT NOT NULL,    -- Secret type (EdDSACurve25519 or ECDSASHA256CurveP256)
    secret      BLOB NOT NULL     -- Secret binary
);

-- This table stores secrets for encrypting / decrypting data
CREATE TABLE x25519_secret
(
    handle BLOB PRIMARY KEY, -- Secret handle
    secret BLOB NOT NULL     -- Secret binary
);


---------------
-- CREDENTIALS
---------------

-- This table stores credentials as received by the application
CREATE TABLE credential
(
    name                  TEXT PRIMARY KEY, -- User-defined name for this credential
    issuer_identifier     TEXT NOT NULL,    -- Identifier of the identity which issued this credential
    issuer_change_history TEXT NOT NULL,    -- Change history of the identity which issued this credential (hex-encoded)
    credential            TEXT NOT NULL     -- Encoded version of the credential: data + signature
);

------------------
-- TRUST CONTEXTS
------------------

CREATE TABLE trust_context
(
    name                     TEXT PRIMARY KEY, -- Name for the trust context
    trust_context_id         TEXT    NOT NULL, -- Identifier for the trust context to be used in policies
    is_default               INTEGER NOT NULL, -- boolean indicating if this trust context is the default one (0 means true)
    credential               TEXT,             -- optional encoded credential which can be retrieved from this trust context
    authority_change_history TEXT,             -- optional: change history of the authority to call for verifying credentials
    authority_route          TEXT              -- optional: route of the authority to call for verifying credentials
);

-- This table stores policies. A policy is an expression which
-- can be evaluated against an environment (a list of name/value pairs)
-- to assess if a given action can be performed on a given resource
CREATE TABLE policy
(
    resource   TEXT NOT NULL, -- resource name
    action     TEXT NOT NULL, -- action name
    expression BLOB NOT NULL  -- encoded expression to evaluate
);

CREATE UNIQUE INDEX policy_index ON policy (resource, action);

---------
-- NODES
---------

-- This table stores information about local nodes
CREATE TABLE node
(
    name                 TEXT PRIMARY KEY, -- Node name
    identifier           TEXT    NOT NULL, -- Identifier of the default identity associated to the node
    verbosity            INTEGER NOT NULL, -- Verbosity level used for logging
    is_default           INTEGER NOT NULL, -- boolean indicating if this node is the default one (0 means true)
    is_authority         INTEGER NOT NULL, -- boolean indicating if this node is an authority node (0 means true). This boolean is used to be able to show an authority node as UP even if its TCP listener cannot be accessed.
    tcp_listener_address TEXT,             -- Socket address for the node default TCP Listener (can be NULL if the node has not been started)
    pid                  INTEGER           -- Current process id of the node if it has been started
);

-- This table stores the project name to use for a given node
CREATE TABLE node_project
(
    node_name    TEXT PRIMARY KEY, -- Node name
    project_name TEXT NOT NULL     -- Project name
);

---------------------------
-- PROJECTS, SPACES, USERS
---------------------------

-- This table store data about projects as returned by the Controller
CREATE TABLE project
(
    project_id             TEXT PRIMARY KEY, -- Identifier of the project
    project_name           TEXT    NOT NULL, -- Name of the project
    is_default             INTEGER NOT NULL, -- boolean indicating if this project is the default one (0 means true)
    space_id               TEXT    NOT NULL, -- Identifier of the space associated to the project
    space_name             TEXT    NOT NULL, -- Name of the space associated to the project
    identifier             TEXT,             -- optional: identifier of the project identity
    access_route           TEXT    NOT NULL, -- Route used to create a secure channel to the project
    authority_identity     TEXT,             -- Encoded identity of the authority associated to the project
    authority_access_route TEXT,             -- Route te the authority associated to the project
    version                TEXT,             -- Orchestrator software version
    running                INTEGER,          -- boolean indicating if this project is currently accessible
    operation_id           TEXT              -- optional id of the operation currently creating the project on the Controller side
);

-- This table provides the list of users associated to a given project
CREATE TABLE user_project
(
    user_email TEXT NOT NULL, -- User email
    project_id TEXT NOT NULL  -- Project id
);

-- This table provides additional information for users associated to a project or a space
CREATE TABLE user_role
(
    user_id    INTEGER NOT NULL, -- User id
    project_id TEXT    NOT NULL, -- Project id
    user_email TEXT    NOT NULL, -- User email
    role       TEXT    NOT NULL, -- Role of the user: admin or member
    scope      TEXT    NOT NULL  -- Scope of the role: space, project, or service
);

-- This table stores data about spaces as returned by the controller
CREATE TABLE space
(
    space_id   TEXT PRIMARY KEY, -- Identifier of the space
    space_name TEXT    NOT NULL, -- Name of the space
    is_default INTEGER NOT NULL  -- boolean indicating if this project is the default one (0 means true)
);

-- This table provides the list of users associated to a given project
CREATE TABLE user_space
(
    user_email TEXT NOT NULL, -- User email
    space_id   TEXT NOT NULL  -- Space id
);

-- This table provides additional information for users after they have been authenticated
CREATE TABLE user
(
    email          TEXT PRIMARY KEY, -- User email
    sub            TEXT    NOT NULL, -- (Sub)ject: unique identifier for the user
    nickname       TEXT    NOT NULL, -- User nickname (or handle)
    name           TEXT    NOT NULL, -- User name
    picture        TEXT    NOT NULL, -- Link to a user picture
    updated_at     TEXT    NOT NULL, -- ISO-8601 date: when this user information was last update
    email_verified INTEGER NOT NULL, -- boolean indicating if the user email has been verified (0 means true)
    is_default     INTEGER NOT NULL  -- boolean indicating if this user is the default user locally (0 means true)
);

---------------
-- APPLICATION
---------------

-- This table stores the current state of an outlet created to expose a service with the desktop application
CREATE TABLE tcp_outlet_status
(
    alias       TEXT PRIMARY KEY, -- Name for the outlet
    socket_addr TEXT NOT NULL,    -- Socket address that the outlet connects to
    worker_addr TEXT NOT NULL,    -- Worker address for the outlet itself
    payload     TEXT              -- Optional status payload
);

-- This table stores the list of services that a user has been invited to connect to
-- via the desktop application
CREATE TABLE incoming_service
(
    invitation_id TEXT PRIMARY KEY, -- Invitation id
    enabled       INTEGER NOT NULL, -- boolean indicating if the user wants to service to be accessible (0 means true)
    name          TEXT NULL         -- Optional user-defined name for the service
);

----------
-- ADDONS
----------

-- This table stores the data necessary to configure the Okta addon
CREATE TABLE okta_config
(
    project_id      TEXT NOT NULL, -- Project id of the project using the addon
    tenant_base_url TEXT NOT NULL, -- Base URL of the tenant
    client_id       TEXT NOT NULL, -- Client id
    certificate     TEXT NOT NULL, -- Certificate
    attributes      TEXT           -- Comma-separated list of attribute names
);

-- This table stores the data necessary to configure the Kafka addons
CREATE TABLE confluent_config
(
    project_id       TEXT NOT NULL, -- Project id of the project using the addon
    bootstrap_server TEXT NOT NULL  -- URL of the bootstrap server
);
