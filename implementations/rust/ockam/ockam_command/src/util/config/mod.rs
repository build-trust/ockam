//! Handle local node configuration

mod atomic;
mod snippets;

use atomic::AtomicUpdater;
use snippets::{ComposableSnippet, Operation};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use slug::slugify;
use std::{
    collections::{BTreeMap, VecDeque},
    fs::{create_dir_all, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, RwLock, RwLockReadGuard},
};

#[derive(Clone)]
pub struct OckamConfig {
    pub(super) dirs: ProjectDirs,
    inner: Arc<RwLock<SyncConfig>>,
}

/// The inner type that actually gets synced to disk
#[derive(Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub api_node: String,
    pub nodes: BTreeMap<String, NodeConfig>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            api_node: "default".into(),
            nodes: BTreeMap::new(),
        }
    }
}

/// A set of errors that occur when trying to update the configuration
///
/// Importantly these errors do not cover the I/O, creation, or
/// saving, only "user error".  While these errors are fatal, they
/// MUST NOT crash the CLI but instead terminate gracefully with a
/// message.
#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("node with name {} already exists", 0)]
    NodeExists(String),
    #[error("node with name {} does not exist", 0)]
    NodeNotFound(String),
}

impl OckamConfig {
    fn get_paths() -> ProjectDirs {
        ProjectDirs::from("io", "ockam", "ockam-cli").expect(
            "failed to determine configuration storage location.
Verify that your XDG_CONFIG_HOME and XDG_DATA_HOME environment variables are correctly set.
Otherwise your OS or OS configuration may not be supported!",
        )
    }

    fn local_data_dir(&self) -> &Path {
        self.dirs.data_local_dir()
    }

    /// Return a static set of config values that can be addressed
    pub fn values() -> Vec<&'static str> {
        vec!["api-node"]
    }

    /// Attempt to load an ockam config.  If none exists, one is
    /// created and then returned.
    pub fn load() -> Self {
        let dirs = Self::get_paths();

        let cfg_dir = dirs.config_dir().to_path_buf();
        if let Err(e) = create_dir_all(&cfg_dir) {
            eprintln!("failed to create configuration directory: {}", e);
            std::process::exit(-1);
        }

        let config_path = cfg_dir.join("config.json");
        let inner = match File::open(&config_path) {
            Ok(ref mut f) => {
                let mut buf = String::new();
                f.read_to_string(&mut buf).expect("failed to read config");
                serde_json::from_str(&buf).unwrap_or_else(|_| {
                    panic!(
                        "failed to parse config.  Try deleting {}",
                        config_path.display()
                    )
                })
            }
            Err(_) => {
                let new_inner = SyncConfig::default();
                let json: String =
                    serde_json::to_string_pretty(&new_inner).expect("failed to serialise config");
                let mut f =
                    File::create(config_path).expect("failed to create default config file");
                f.write_all(json.as_bytes())
                    .expect("failed to write config");
                new_inner
            }
        };

        Self {
            dirs,
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    /// Atomically update the configuration
    pub fn atomic_update(&self) -> AtomicUpdater {
        AtomicUpdater::new(self.dirs.clone(), self.inner.clone())
    }

    ///////////////////// READ ACCESSORS //////////////////////////////

    /// Get the current value of the API node
    pub fn get_api_node(&self) -> String {
        self.inner.read().unwrap().api_node.clone()
    }

    /// Get the node state directory
    pub fn get_node_dir(&self, name: &str) -> Result<PathBuf, ConfigError> {
        let inner = self.inner.read().unwrap();
        let n = inner
            .nodes
            .get(name)
            .ok_or_else(|| ConfigError::NodeNotFound(name.to_string()))?;
        Ok(PathBuf::new().join(&n.state_dir))
    }

    /// Get the API port used by a node
    pub fn get_node_port(&self, name: &str) -> Result<u16, ConfigError> {
        let inner = self.inner.read().unwrap();
        Ok(inner
            .nodes
            .get(name)
            .ok_or_else(|| ConfigError::NodeNotFound(name.to_string()))?
            .port)
    }

    /// In the future this will actually refer to the watchdog pid or
    /// no pid at all but we'll see
    pub fn get_node_pid(&self, name: &str) -> Result<Option<i32>, ConfigError> {
        let inner = self.inner.read().unwrap();
        Ok(inner
            .nodes
            .get(name)
            .ok_or_else(|| ConfigError::NodeNotFound(name.to_string()))?
            .pid)
    }

    /// Check whether another node has been registered with this API
    /// port.  This doesn't catch all port collision errors, but will
    /// get us most of the way there in terms of starting a new node.
    pub fn port_is_used(&self, port: u16) -> bool {
        let inner = self.inner.read().unwrap();

        inner.nodes.iter().any(|(_, n)| n.port == port)
    }

    /// Get only a single node configuration
    pub fn get_node(&self, node: &str) -> Result<NodeConfig, ConfigError> {
        let inner = self.inner.read().unwrap();
        inner
            .nodes
            .get(node)
            .map(Clone::clone)
            .ok_or_else(|| ConfigError::NodeNotFound(node.into()))
    }

    /// Get the current version the selected node configuration
    pub fn select_node<'a>(&'a self, o: &'a Option<String>) -> Option<NodeConfig> {
        let inner = self.inner.read().unwrap();
        inner
            .nodes
            .get(o.as_ref().unwrap_or(&inner.api_node))
            .map(Clone::clone)
    }

    /// Get the log path for a specific node
    ///
    /// The convention is to name the main log `node-name.log` and the
    /// supplimentary log `nod-name.log.stderr`
    pub fn log_paths_for_node(&self, node_name: &String) -> Option<(PathBuf, PathBuf)> {
        let inner = self.inner.read().unwrap();

        let base = &inner.nodes.get(node_name)?.state_dir;
        // TODO: sluggify node names
        Some((
            base.join(format!("{}.log", node_name)),
            base.join(format!("{}.log.stderr", node_name)),
        ))
    }

    /// Get read access to the inner raw configuration
    pub fn get_inner(&self) -> RwLockReadGuard<'_, SyncConfig> {
        self.inner.read().unwrap()
    }

    ///////////////////// WRITE ACCESSORS //////////////////////////////

    /// Add a new node to the configuration for future lookup
    pub fn create_node(&self, name: &str, port: u16, pid: i32) -> Result<(), ConfigError> {
        let mut inner = self.inner.write().unwrap();

        if inner.nodes.contains_key(name) {
            return Err(ConfigError::NodeExists(name.to_string()));
        }

        // Setup logging directory and store it
        let state_dir = self
            .local_data_dir()
            .join(slugify(&format!("node-{}", name)));

        if let Err(e) = create_dir_all(&state_dir) {
            eprintln!("failed to create new node state directory: {}", e);
            std::process::exit(-1);
        }

        inner.nodes.insert(
            name.to_string(),
            NodeConfig {
                port,
                state_dir,
                pid: Some(pid),
                composites: vec![ComposableSnippet {
                    id: "_start".into(),
                    op: Operation::Node {
                        port,
                        node_name: name.to_string(),
                    },
                    params: vec![],
                }]
                .into(),
            },
        );
        Ok(())
    }

    /// Delete an existing node
    pub fn delete_node(&self, name: &str) -> Result<(), ConfigError> {
        let mut inner = self.inner.write().unwrap();
        match inner.nodes.remove(name) {
            Some(_) => Ok(()),
            None => Err(ConfigError::NodeExists(name.to_string())),
        }
    }

    /// Update the pid of an existing node process
    pub fn update_pid(&self, name: &str, pid: impl Into<Option<i32>>) -> Result<(), ConfigError> {
        let mut inner = self.inner.write().unwrap();

        if !inner.nodes.contains_key(name) {
            return Err(ConfigError::NodeNotFound(name.to_string()));
        }

        inner.nodes.get_mut(name).unwrap().pid = pid.into();
        Ok(())
    }

    /// Update the api node name on record
    pub fn set_api_node(&self, node_name: &str) {
        let mut inner = self.inner.write().unwrap();
        inner.api_node = node_name.into();
    }

    ///////////////////// COMPOSITION CONSTRUCTORS //////////////////////////////

    pub fn add_transport(&self, node: &str, listen: bool, tcp: bool, addr: String) {
        let mut inner = self.inner.write().unwrap();
        inner
            .nodes
            .get_mut(node)
            .unwrap()
            .composites
            .push_back(ComposableSnippet {
                id: format!(
                    "_transport_{}_{}_{}",
                    if listen { "listen" } else { "connect" },
                    if tcp { "tcp" } else { "unknown" },
                    addr
                ),
                op: Operation::Transport { listen, tcp, addr },
                params: vec![],
            })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeConfig {
    pub port: u16,
    pub pid: Option<i32>,
    pub state_dir: PathBuf,
    pub composites: VecDeque<ComposableSnippet>,
}
