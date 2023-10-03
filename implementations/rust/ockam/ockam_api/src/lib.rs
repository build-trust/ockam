//! This crate supports the creation of a fully-featured Ockam Node
//! (see [`NodeManager`](https://github.com/build-trust/ockam/blob/2fc6d7714a4e54f8734c172ad6480fedc6e3629c/implementations/rust/ockam/ockam_api/src/nodes/service.rs#L87) in [`src/nodes/service.rs`](https://github.com/build-trust/ockam/blob/2fc6d7714a4e54f8734c172ad6480fedc6e3629c/implementations/rust/ockam/ockam_api/src/nodes/service.rs)).
//!
//! # Configuration
//!
//! A `NodeManager` maintains its configuration as a list of directories and files stored under
//! the `OCKAM_HOME` directory (`~/.ockam`) by default:
//! ```shell
//! root
//! ├─ credentials
//! │  ├─ c1.json
//! │  ├─ c2.json
//! │  └─ ...
//! ├─ defaults
//! │  ├── credential -> ...
//! │  ├── identity -> ...
//! │  ├── node -> ...
//! │  └── vault -> ...
//! ├─ identities
//! │  ├─ data
//! │  │  ├─ authenticated-storage.lmdb
//! │  │  └─ authenticated-storage.lmdb-lock
//! │  ├─ identity1.json
//! │  ├─ identity2.json
//! │  └─ ...
//! ├─ nodes
//! │  ├─ node1
//! │  │  ├─ default_identity -> ...
//! │  │  ├─ default_vault -> ...
//! │  │  ├─ policies-storage.lmdb
//! │  │  ├─ policies-storage.lmdb-lock
//! │  │  ├─ setup.json
//! │  │  ├─ stderr.log
//! │  │  ├─ stdout.log
//! │  │  └─ version.log
//! │  ├─ node2
//! │  └─ ...
//! ├─ projects
//! │  └─ default.json
//! ├─ trust_contexts
//! │  └─ default.json
//! └─ vaults
//!    ├─ vault1.json
//!    ├─ vault2.json
//!    ├─ ...
//!    └─ data
//!       ├─ vault1.lmdb
//!       ├─ vault1.lmdb-lock
//!       ├─ vault2.lmdb
//!       ├─ vault2.lmdb-lock
//!       └─ ...
//! ```
//! # `credentials`
//!
//! Each file stored under the `credentials` directory contains the credential for a given identity.
//! Those files are created with the `ockam credential store` command. They are then read during the creation of
//! a secure channel to send the credentials to the other party
//!
//! # `defaults`
//!
//! This directory contains symlinks to other files or directories in order to specify which node,
//! identity, credential or vault must be considered as a default when running a command expecting those
//! inputs
//!
//! # `identities`
//!
//! This directory contains one file per identity and a data directory. An identity file is created
//! with the `ockam identity create` command or created by default for some commands (in that case the
//! `defaults/identity` symlink points to that identity). The identity file contains:
//!
//! - the identity identifier
//! - the enrollment status for that identity
//!
//! The `data` directory contains a LMDB database with other information about identities:
//!  - the credential attributes that have been verified for this identity. Those attributes are
//!    generally used in ABAC rules that are specified on secure channels. For example when sending messages
//!    via a secure channel and using the Orchestrator the `project` attribute will be checked and the LMDB database accessed
//!
//!  - the list of key changes for each identity. These key changes are created (or updated) when an identity
//!    is created either by using the command line or by using the identity service.
//!    The key changes are accessed in order to get the latest public key associated to a given identity
//!    when checking its signature during the creation of a secure channel.
//!    They are also accessed to retrieve the key id associated to that key and then use a Vault to create a signature
//!    for an identity
//!
//! Note: for each `.lmdb` file there is a corresponding `lmdb-lock` file which is used to control
//! the exclusive access to the LMDB database even if several OS processes are trying to modify it.
//! For example when several nodes are started using the same `NodeManager`.
//!
//! # `nodes`
//!
//! This directory contains:
//!
//!  - symlinks to default values for the node: identity and vault
//!  - a database for ABAC policies
//!  - a setup file containing some configuration information for the node (is it an authority node?, what is the TCP listener address?,...).
//!    That file is created when a node is created and read again if the node is restarted
//!  - log files: for system errors and system outputs. The stdout.log file is where almost all the node logs are written
//!  - a version number for the configuration
//!
//! # `projects`
//!
//! This directory contains a list of files, one per project that was created, either the default project
//! or via the `ockam project create` command. A project file contains:
//!
//!  - the project identifier and the space it belongs to
//!  - the authority used by that project (identity, route)
//!  - the configuration for the project plugins
//!
//! # `trust_context`
//!
//! This directory contains a list of files, one per trust context. A trust context can created with
//! the `ockam trust_context create` command. It can then be referred to during the creation of a
//! secure channel as a way to specify which authority can attest to the validity of which attributes
//!
//! # `vaults`
//!
//! This directory contains one file per vault that is either created by default or with the `ockam vault create`
//! command. That file contains the configuration for the vault, which for now consists only in
//! declaring if the vault is backed by an AWS KMS or not.
//!
//! The rest of the vault data is stored in an LMDB database under the `data` directory with one `.lmdb`
//! file per vault. A vault contains secrets which are generally used during the creation of secure
//! channels to sign or encrypt data involved in the handshake.
//!
pub mod address;
pub mod auth;
pub mod authenticator;
pub mod bootstrapped_identities_store;
pub mod cli_state;
pub mod cloud;
pub mod config;
pub mod echoer;
pub mod enroll;
pub mod error;
pub mod hop;
pub mod identity;
pub mod kafka;
pub mod minicbor_url;
pub mod nodes;
pub mod okta;
pub mod port_range;
pub mod trust_context;
pub mod uppercase;

pub mod authority_node;
mod influxdb_token_lease;

mod schema;
mod session;
mod util;

pub use influxdb_token_lease::*;
pub use util::*;

#[macro_use]
extern crate tracing;

pub struct DefaultAddress;

impl DefaultAddress {
    pub const AUTHENTICATED_SERVICE: &'static str = "authenticated";
    pub const RELAY_SERVICE: &'static str = "forwarding_service";
    pub const UPPERCASE_SERVICE: &'static str = "uppercase";
    pub const ECHO_SERVICE: &'static str = "echo";
    pub const HOP_SERVICE: &'static str = "hop";
    pub const CREDENTIALS_SERVICE: &'static str = "credentials";
    pub const SECURE_CHANNEL_LISTENER: &'static str = "api";
    pub const DIRECT_AUTHENTICATOR: &'static str = "direct_authenticator";
    pub const CREDENTIAL_ISSUER: &'static str = "credential_issuer";
    pub const ENROLLMENT_TOKEN_ISSUER: &'static str = "enrollment_token_issuer";
    pub const ENROLLMENT_TOKEN_ACCEPTOR: &'static str = "enrollment_token_acceptor";
    pub const OKTA_IDENTITY_PROVIDER: &'static str = "okta";
    pub const KAFKA_OUTLET: &'static str = "kafka_outlet";
    pub const KAFKA_CONSUMER: &'static str = "kafka_consumer";
    pub const KAFKA_PRODUCER: &'static str = "kafka_producer";
    pub const KAFKA_DIRECT: &'static str = "kafka_direct";

    pub fn is_valid(name: &str) -> bool {
        matches!(
            name,
            Self::AUTHENTICATED_SERVICE
                | Self::RELAY_SERVICE
                | Self::UPPERCASE_SERVICE
                | Self::ECHO_SERVICE
                | Self::HOP_SERVICE
                | Self::CREDENTIALS_SERVICE
                | Self::SECURE_CHANNEL_LISTENER
                | Self::DIRECT_AUTHENTICATOR
                | Self::CREDENTIAL_ISSUER
                | Self::ENROLLMENT_TOKEN_ISSUER
                | Self::ENROLLMENT_TOKEN_ACCEPTOR
                | Self::OKTA_IDENTITY_PROVIDER
                | Self::KAFKA_CONSUMER
                | Self::KAFKA_PRODUCER
                | Self::KAFKA_OUTLET
                | Self::KAFKA_DIRECT
        )
    }

    pub fn iter() -> impl Iterator<Item = &'static str> {
        [
            Self::AUTHENTICATED_SERVICE,
            Self::RELAY_SERVICE,
            Self::UPPERCASE_SERVICE,
            Self::ECHO_SERVICE,
            Self::HOP_SERVICE,
            Self::CREDENTIALS_SERVICE,
            Self::SECURE_CHANNEL_LISTENER,
            Self::DIRECT_AUTHENTICATOR,
            Self::CREDENTIAL_ISSUER,
            Self::ENROLLMENT_TOKEN_ISSUER,
            Self::ENROLLMENT_TOKEN_ACCEPTOR,
            Self::OKTA_IDENTITY_PROVIDER,
            Self::KAFKA_CONSUMER,
            Self::KAFKA_PRODUCER,
            Self::KAFKA_OUTLET,
            Self::KAFKA_DIRECT,
        ]
        .iter()
        .copied()
    }
}

pub mod actions {
    use ockam_abac::Action;

    pub const HANDLE_MESSAGE: Action = Action::assert_inline("handle_message");
}

pub mod resources {
    use ockam_abac::Resource;

    pub const INLET: Resource = Resource::assert_inline("tcp-inlet");
    pub const OUTLET: Resource = Resource::assert_inline("tcp-outlet");
}

use core::fmt;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Newtype around [`Vec<u8>`] that provides base-16 string encoding using serde.
#[derive(Debug, Clone, Default, Encode, Decode)]
#[cbor(transparent)]
pub struct HexByteVec(#[n(0)] pub Vec<u8>);

impl HexByteVec {
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for HexByteVec {
    fn from(v: Vec<u8>) -> Self {
        Self(v)
    }
}

impl From<HexByteVec> for Vec<u8> {
    fn from(h: HexByteVec) -> Self {
        h.0
    }
}

impl Serialize for HexByteVec {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            hex::serde::serialize(&*self.0, s)
        } else {
            s.serialize_bytes(&self.0)
        }
    }
}

impl<'de> Deserialize<'de> for HexByteVec {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if d.is_human_readable() {
            let v: Vec<u8> = hex::serde::deserialize(d)?;
            Ok(Self(v))
        } else {
            let v = <Vec<u8>>::deserialize(d)?;
            Ok(Self(v))
        }
    }
}

impl fmt::Display for HexByteVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.serialize(f)
    }
}

#[cfg(test)]
mod test {
    use super::DefaultAddress;

    #[test]
    fn test_default_address_is_valid() {
        assert!(!DefaultAddress::is_valid("foo"));
        assert!(DefaultAddress::is_valid(
            DefaultAddress::AUTHENTICATED_SERVICE
        ));
        assert!(DefaultAddress::is_valid(DefaultAddress::RELAY_SERVICE));
        assert!(DefaultAddress::is_valid(DefaultAddress::UPPERCASE_SERVICE));
        assert!(DefaultAddress::is_valid(DefaultAddress::ECHO_SERVICE));
        assert!(DefaultAddress::is_valid(DefaultAddress::HOP_SERVICE));
        assert!(DefaultAddress::is_valid(
            DefaultAddress::CREDENTIALS_SERVICE
        ));
        assert!(DefaultAddress::is_valid(
            DefaultAddress::SECURE_CHANNEL_LISTENER
        ));
        assert!(DefaultAddress::is_valid(
            DefaultAddress::DIRECT_AUTHENTICATOR
        ));
        assert!(DefaultAddress::is_valid(DefaultAddress::CREDENTIAL_ISSUER));
        assert!(DefaultAddress::is_valid(
            DefaultAddress::ENROLLMENT_TOKEN_ISSUER
        ));
        assert!(DefaultAddress::is_valid(
            DefaultAddress::ENROLLMENT_TOKEN_ACCEPTOR
        ));
        assert!(DefaultAddress::is_valid(
            DefaultAddress::OKTA_IDENTITY_PROVIDER
        ));
        assert!(DefaultAddress::is_valid(DefaultAddress::KAFKA_CONSUMER));
        assert!(DefaultAddress::is_valid(DefaultAddress::KAFKA_PRODUCER));
    }
}
