//! Configuration files used by the ockam CLI

use crate::config::{
    lookup::{ConfigLookup, InternetAddress},
    snippet::ComposableSnippet,
    ConfigValues,
};
use crate::HexByteVec;
use directories::ProjectDirs;
use ockam_core::Result;
use ockam_identity::{IdentityIdentifier, IdentityVault, PublicIdentity};
use ockam_multiaddr::MultiAddr;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
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
    pub nodes: BTreeMap<String, NodeConfig>,

    #[serde(default = "default_lookup")]
    pub lookup: ConfigLookup,

    pub default_identity: Option<Vec<u8>>,
    pub default_vault_path: Option<PathBuf>,
    /// Default node
    pub default: Option<String>,
}

fn default_nodes() -> BTreeMap<String, NodeConfig> {
    BTreeMap::new()
}

fn default_lookup() -> ConfigLookup {
    ConfigLookup::default()
}

impl ConfigValues for OckamConfig {
    fn default_values(_node_dir: &Path) -> Self {
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

    // UTILITY FUNCTIONS NEEDED IN OCKAM_API

    /// This function could be zero-copy if we kept the lock on the
    /// backing store for as long as we needed it.  Because this may
    /// have unwanted side-effects, instead we eagerly copy data here.
    /// This may be optimised in the future!
    pub fn get_lookup(&self) -> &ConfigLookup {
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
pub struct NodeConfig {
    #[serde(default = "default_name")]
    pub name: String,

    #[serde(default = "default_addr")]
    pub addr: InternetAddress,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_verbose")]
    pub verbose: u8,

    pub pid: Option<i32>,
    pub state_dir: Option<PathBuf>,
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

/// Node launch configuration
///
///
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StartupConfig {
    pub commands: VecDeque<ComposableSnippet>,
}

impl ConfigValues for StartupConfig {
    fn default_values(_node_dir: &Path) -> Self {
        Self::default()
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
    fn default_values(_: &Path) -> Self {
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
