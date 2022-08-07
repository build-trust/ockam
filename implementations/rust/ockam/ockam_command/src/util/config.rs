//! Handle local node configuration

use ockam_api::config::{cli, Config};
use slug::slugify;
use std::{fs::create_dir_all, ops::Deref, path::PathBuf, sync::RwLockReadGuard};

pub use ockam_api::config::cli::NodeConfig;
pub use ockam_api::config::snippet::{
    ComposableSnippet, Operation, PortalMode, Protocol, RemoteMode,
};

/// A simple wrapper around the main configuration structure to add
/// local config utility/ query functions
#[derive(Clone)]
pub struct OckamConfig {
    inner: Config<cli::OckamConfig>,
}

impl Deref for OckamConfig {
    type Target = Config<cli::OckamConfig>;

    fn deref(&self) -> &Self::Target {
        &self.inner
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
    pub fn load() -> Self {
        let directories = cli::OckamConfig::directories();
        let config_path = directories.config_dir().join("config.json");
        let inner = Config::<cli::OckamConfig>::load(config_path);
        inner.writelock_inner().directories = Some(directories);
        Self { inner }
    }

    /// Get available global configuration values
    // TODO: make this consider node scope options
    pub fn values() -> Vec<&'static str> {
        vec!["api-node"]
    }

    /// Get the current API node configuration setting
    pub fn get_api_node(&self) -> String {
        self.inner.readlock_inner().api_node.clone()
    }

    /// Get the node state directory
    pub fn get_node_dir(&self, name: &str) -> Result<PathBuf, ConfigError> {
        let inner = self.inner.readlock_inner();
        let n = inner
            .nodes
            .get(name)
            .ok_or_else(|| ConfigError::NodeNotFound(name.to_string()))?;
        Ok(PathBuf::new().join(&n.state_dir))
    }

    /// Get the API port used by a node
    pub fn get_node_port(&self, name: &str) -> u16 {
        let inner = self.inner.readlock_inner();
        inner
            .nodes
            .get(name)
            .unwrap_or_else(|| {
                eprintln!("No such node available. Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            })
            .port
    }

    /// In the future this will actually refer to the watchdog pid or
    /// no pid at all but we'll see
    pub fn get_node_pid(&self, name: &str) -> Result<Option<i32>, ConfigError> {
        let inner = self.inner.readlock_inner();
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
        let inner = self.inner.readlock_inner();

        inner.nodes.iter().any(|(_, n)| n.port == port)
    }

    /// Get only a single node configuration
    pub fn get_node(&self, node: &str) -> Result<NodeConfig, ConfigError> {
        let inner = self.inner.readlock_inner();
        inner
            .nodes
            .get(node)
            .map(Clone::clone)
            .ok_or_else(|| ConfigError::NodeNotFound(node.into()))
    }

    /// Get the current version the selected node configuration
    pub fn select_node<'a>(&'a self, o: &'a str) -> Option<NodeConfig> {
        let inner = self.inner.readlock_inner();
        inner.nodes.get(o).map(Clone::clone)
    }

    /// Get the log path for a specific node
    ///
    /// The convention is to name the main log `node-name.log` and the
    /// supplementary log `nod-name.log.stderr`
    pub fn log_paths_for_node(&self, node_name: &String) -> Option<(PathBuf, PathBuf)> {
        let inner = self.inner.readlock_inner();

        let base = &inner.nodes.get(node_name)?.state_dir;
        // TODO: sluggify node names
        Some((
            base.join(format!("{}.log", node_name)),
            base.join(format!("{}.log.stderr", node_name)),
        ))
    }

    /// Get read access to the inner raw configuration
    pub fn get_inner(&self) -> RwLockReadGuard<'_, cli::OckamConfig> {
        self.inner.readlock_inner()
    }

    /// Get the launch configuration for a node
    pub fn get_launch_config(&self, name: &str) -> Result<StartupConfig, ConfigError> {
        let path = self.get_node_dir(name)?;
        Ok(StartupConfig::load(path))
    }

    ///////////////////// WRITE ACCESSORS //////////////////////////////

    /// Add a new node to the configuration for future lookup
    pub fn create_node(&self, name: &str, port: u16) -> Result<(), ConfigError> {
        let mut inner = self.inner.writelock_inner();

        if inner.nodes.contains_key(name) {
            return Err(ConfigError::NodeExists(name.to_string()));
        }

        // Setup logging directory and store it
        let state_dir = inner
            .directories
            .as_ref()
            .expect("configuration is in an invalid state")
            .data_local_dir()
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
                pid: Some(0),
            },
        );
        Ok(())
    }

    /// Delete an existing node
    pub fn delete_node(&self, name: &str) -> Result<(), ConfigError> {
        let mut inner = self.inner.writelock_inner();
        match inner.nodes.remove(name) {
            Some(_) => Ok(()),
            None => Err(ConfigError::NodeExists(name.to_string())),
        }
    }

    /// Update the pid of an existing node process
    pub fn update_pid(&self, name: &str, pid: impl Into<Option<i32>>) -> Result<(), ConfigError> {
        let mut inner = self.inner.writelock_inner();

        if !inner.nodes.contains_key(name) {
            return Err(ConfigError::NodeNotFound(name.to_string()));
        }

        inner.nodes.get_mut(name).unwrap().pid = pid.into();
        Ok(())
    }

    /// Update the api node name on record
    pub fn set_api_node(&self, node_name: &str) {
        let mut inner = self.inner.writelock_inner();
        inner.api_node = node_name.into();
    }
}

/// A simple wrapper around the main configuration structure to add
/// local config utility/ query functions
pub struct StartupConfig {
    inner: Config<cli::StartupConfig>,
}

impl Deref for StartupConfig {
    type Target = Config<cli::StartupConfig>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl StartupConfig {
    pub fn load(node_dir: PathBuf) -> Self {
        let config_path = node_dir.join("startup.json");
        let inner = Config::<cli::StartupConfig>::load(config_path);
        Self { inner }
    }

    /// Add a new composite command to a node
    pub fn add_composite(&self, composite: ComposableSnippet) {
        let mut inner = self.inner.writelock_inner();
        inner.commands.push_back(composite);
    }
}
