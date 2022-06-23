//! Handle local node configuration

mod atomic;

use atomic::AtomicUpdater;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use slug::slugify;
use std::{
    collections::BTreeMap,
    fs::{create_dir_all, File},
    io::{Read, Write},
    ops::Deref,
    path::{Path, PathBuf},
};

/// Wraps around ProjectDirs in a serde friendly manner
#[derive(Clone)]
struct OckamDirectories(ProjectDirs);

impl Default for OckamDirectories {
    fn default() -> Self {
        Self(OckamConfig::get_paths())
    }
}

impl Deref for OckamDirectories {
    type Target = ProjectDirs;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OckamConfig {
    #[serde(skip, default)]
    #[allow(unused)]
    dirs: OckamDirectories,
    pub api_node: String,
    nodes: BTreeMap<String, NodeConfig>,
}

impl Default for OckamConfig {
    fn default() -> Self {
        Self {
            dirs: OckamDirectories::default(),
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

    fn config_dir(&self) -> &Path {
        self.dirs.config_dir()
    }

    fn local_data_dir(&self) -> &Path {
        self.dirs.data_local_dir()
    }

    /// Return a static set of config values that can be addressed
    pub fn values() -> Vec<&'static str> {
        vec!["api-node", "log-path"]
    }

    /// Attempt to load an ockam config.  If none exists, one is
    /// created and then returned.
    pub fn load() -> Self {
        let cfg_dir = Self::default().config_dir().to_path_buf();
        if let Err(e) = create_dir_all(&cfg_dir) {
            eprintln!("failed to create configuration directory: {}", e);
            std::process::exit(-1);
        }

        let config_path = cfg_dir.join("config.json");
        match File::open(&config_path) {
            Ok(ref mut f) => {
                let mut buf = String::new();
                f.read_to_string(&mut buf).expect("failed to read config");
                serde_json::from_str(&buf).expect("failed to parse config.  Try deleting the file $HOME/.config/ockam-cli/config.json")
            }
            Err(_) => {
                let new = Self::default();
                let json: String =
                    serde_json::to_string_pretty(&new).expect("failed to serialise config");
                let mut f =
                    File::create(config_path).expect("failed to create default config file");
                f.write_all(json.as_bytes())
                    .expect("failed to write config");
                new
            }
        }
    }

    /// Atomically update the configuration
    pub fn atomic_update(&self) -> AtomicUpdater {
        AtomicUpdater::new(self)
    }

    /// Add a new node to the configuration for future lookup
    pub fn create_node(&mut self, name: &str, port: u16, pid: i32) -> Result<(), ConfigError> {
        if self.nodes.contains_key(name) {
            return Err(ConfigError::NodeExists(name.to_string()));
        }

        // Setup logging directory and store it
        let log_dir = self
            .local_data_dir()
            .join(slugify(&format!("node-{}", name)));

        if let Err(e) = create_dir_all(&log_dir) {
            eprintln!("failed to create new node state directory: {}", e);
            std::process::exit(-1);
        }

        self.nodes.insert(
            name.to_string(),
            NodeConfig {
                port,
                log_dir,
                pid: Some(pid),
            },
        );
        Ok(())
    }

    /// Delete an existing node
    pub fn delete_node(&mut self, name: &str) -> Result<(), ConfigError> {
        match self.nodes.remove(name) {
            Some(_) => Ok(()),
            None => Err(ConfigError::NodeExists(name.to_string())),
        }
    }

    /// Update the pid of an existing node process
    pub fn update_pid(
        &mut self,
        name: &str,
        pid: impl Into<Option<i32>>,
    ) -> Result<(), ConfigError> {
        if !self.nodes.contains_key(name) {
            return Err(ConfigError::NodeNotFound(name.to_string()));
        }

        self.nodes.get_mut(name).unwrap().pid = pid.into();
        Ok(())
    }

    /// Check whether another node has been registered with this API
    /// port.  This doesn't catch all port collision errors, but will
    /// get us most of the way there in terms of starting a new node.
    pub fn port_is_used(&self, port: u16) -> bool {
        self.nodes.iter().any(|(_, n)| n.port == port)
    }

    /// Get read access to all node configuration
    pub fn get_nodes(&self) -> &BTreeMap<String, NodeConfig> {
        &self.nodes
    }

    /// Get the selected node configuration
    pub fn select_node<'a>(&'a self, o: &'a Option<String>) -> Option<&'a NodeConfig> {
        self.nodes.get(o.as_ref().unwrap_or(&self.api_node))
    }

    /// Get the log path for a specific node
    ///
    /// The convention is to name the main log `node-name.log` and the
    /// supplimentary log `nod-name.log.stderr`
    pub fn log_paths_for_node(&self, node_name: &String) -> Option<(PathBuf, PathBuf)> {
        let base = &self.nodes.get(node_name)?.log_dir;
        // TODO: sluggify node names
        Some((
            base.join(format!("{}.log", node_name)),
            base.join(format!("{}.log.stderr", node_name)),
        ))
    }

    /// Update the api node name on record
    pub fn set_api_node(&mut self, node_name: &str) {
        self.api_node = node_name.into();
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeConfig {
    pub port: u16,
    pub pid: Option<i32>,
    pub log_dir: PathBuf,
}
