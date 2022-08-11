//! Configuration files used by the ockam CLI

use crate::config::{snippet::ComposableSnippet, ConfigValues};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::net::{SocketAddrV4, SocketAddrV6};
use std::{
    env,
    net::SocketAddr,
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
    pub api_node: String,
    pub nodes: BTreeMap<String, NodeConfig>,
    pub default_identity: Option<Vec<u8>>,
    pub default_vault_path: Option<PathBuf>,
}

impl ConfigValues for OckamConfig {
    fn default_values(_node_dir: &Path) -> Self {
        Self {
            directories: Some(Self::directories()),
            api_node: "default".into(),
            nodes: BTreeMap::new(),
            default_identity: None,
            default_vault_path: None,
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
    pub fn build_lookup(&self) -> BTreeMap<String, SocketAddr> {
        self.nodes
            .iter()
            .map(|(name, cfg)| (name.clone(), cfg.bind.parse().unwrap()))
            .collect()
    }
}

/// An internet address abstraction (v6/v4/dns)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InternetAddress {
    /// DNSaddr and port
    Dns(String, u16),
    /// An IPv4 socket address
    V4(SocketAddrV4),
    /// An IPv6 socket address
    V6(SocketAddrV6),
}

impl InternetAddress {
    pub fn from_dns(s: String, port: u16) -> Self {
        Self::Dns(s, port)
    }

    /// Get the port for this address
    pub fn port(&self) -> u16 {
        match self {
            Self::Dns(_, port) => *port,
            Self::V4(v4) => v4.port(),
            Self::V6(v6) => v6.port(),
        }
    }
}

impl From<SocketAddr> for InternetAddress {
    fn from(sa: SocketAddr) -> Self {
        match sa {
            SocketAddr::V4(v4) => Self::V4(v4),
            SocketAddr::V6(v6) => Self::V6(v6),
        }
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
    pub remote: bool,
    pub bind: String,
    pub port: u16,
    pub verbose: u8,
    pub pid: Option<i32>,
    pub state_dir: Option<PathBuf>,
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
