//! Configuration files used by the ockam CLI

use crate::config::{
    lookup::{ConfigLookup, InternetAddress},
    ConfigValues,
};
use crate::HexByteVec;
use directories::ProjectDirs;
use ockam_core::Result;
use ockam_identity::{IdentityIdentifier, IdentityVault, PublicIdentity};
use ockam_multiaddr::MultiAddr;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::{
    env,
    path::{Path, PathBuf},
};

/// The main ockam CLI configuration
///
/// Used to determine CLI runtime behaviour and index existing nodes
/// on a system.
///
/// ## Updates
///
/// This configuration is read and updated by the user-facing `ockam`
/// CLI.  Furthermore the data is only relevant for user-facing
/// `ockam` CLI instances.  As such writes to this config don't have
/// to be synchronised to detached consumers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OckamConfig {
    /// We keep track of the project directories at runtime but don't
    /// persist this data to the configuration
    #[serde(skip)]
    pub directories: Option<ProjectDirs>,
    #[serde(default = "default_nodes")]
    pub nodes: BTreeMap<String, NodeConfigOld>,

    #[serde(default = "default_lookup")]
    pub lookup: ConfigLookup,

    pub default_identity: Option<Vec<u8>>,
    pub default_vault_path: Option<PathBuf>,
    /// Default node
    pub default: Option<String>,
}

fn default_nodes() -> BTreeMap<String, NodeConfigOld> {
    BTreeMap::new()
}

fn default_lookup() -> ConfigLookup {
    ConfigLookup::default()
}

impl ConfigValues for OckamConfig {
    fn default_values() -> Self {
        Self {
            directories: Some(Self::directories()),
            nodes: BTreeMap::new(),
            lookup: default_lookup(),
            default_identity: None,
            default_vault_path: None,
            default: None,
        }
    }
}

impl OckamConfig {
    /// Determine the default storage location for the ockam config
    pub fn directories() -> ProjectDirs {
        match env::var("OCKAM_PROJECT_PATH") {
            Ok(dir) => {
                let dir = PathBuf::from(&dir);
                ProjectDirs::from_path(dir).expect(
                    "failed to determine configuration storage location.
Verify that your OCKAM_PROJECT_PATH environment variable is valid.",
                )
            }
            Err(_) => ProjectDirs::from("io", "ockam", "ockam-cli").expect(
                "failed to determine configuration storage location.
Verify that your XDG_CONFIG_HOME and XDG_DATA_HOME environment variables are correctly set.
Otherwise your OS or OS configuration may not be supported!",
            ),
        }
    }

    /// This function could be zero-copy if we kept the lock on the
    /// backing store for as long as we needed it.  Because this may
    /// have unwanted side-effects, instead we eagerly copy data here.
    /// This may be optimised in the future!
    pub fn lookup(&self) -> &ConfigLookup {
        &self.lookup
    }
}

/// Per-node runtime configuration
///
/// ## Updates
///
/// This configuration is used to keep track of individual nodes by
/// the CLI.  The config is updated periodically but writes to it
/// don't have to be synced to consumers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeConfigOld {
    #[serde(default = "default_name")]
    name: String,
    #[serde(default = "default_addr")]
    addr: InternetAddress,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_verbose")]
    verbose: u8,
    pub pid: Option<i32>,
    state_dir: Option<PathBuf>,
}

fn default_name() -> String {
    String::new()
}
fn default_addr() -> InternetAddress {
    InternetAddress::default()
}
fn default_port() -> u16 {
    InternetAddress::default().port()
}
fn default_verbose() -> u8 {
    0
}

impl NodeConfigOld {
    pub fn new(
        name: String,
        addr: InternetAddress,
        port: u16,
        verbose: u8,
        pid: Option<i32>,
        state_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            name,
            addr,
            port,
            verbose,
            pid,
            state_dir,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn addr(&self) -> &InternetAddress {
        &self.addr
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn verbose(&self) -> u8 {
        self.verbose
    }

    pub fn pid(&self) -> Option<i32> {
        self.pid
    }

    pub fn state_dir(&self) -> Option<&Path> {
        self.state_dir.as_deref()
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AuthoritiesConfig {
    authorities: BTreeMap<IdentityIdentifier, Authority>,
}

impl AuthoritiesConfig {
    pub fn add_authority(&mut self, i: IdentityIdentifier, a: Authority) {
        self.authorities.insert(i, a);
    }

    pub fn authorities(&self) -> impl Iterator<Item = (&IdentityIdentifier, &Authority)> {
        self.authorities.iter()
    }

    pub async fn to_public_identities<V>(&self, vault: &V) -> Result<Vec<PublicIdentity>>
    where
        V: IdentityVault,
    {
        let mut v = Vec::new();
        for a in self.authorities.values() {
            v.push(PublicIdentity::import(a.identity.as_slice(), vault).await?)
        }
        Ok(v)
    }
}

impl ConfigValues for AuthoritiesConfig {
    fn default_values() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Authority {
    identity: HexByteVec,
    access: MultiAddr,
}

impl Authority {
    pub fn new(identity: Vec<u8>, addr: MultiAddr) -> Self {
        Self {
            identity: identity.into(),
            access: addr,
        }
    }

    pub fn identity(&self) -> &[u8] {
        self.identity.as_slice()
    }

    pub fn access_route(&self) -> &MultiAddr {
        &self.access
    }
}
