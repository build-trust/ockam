//! Handle local node configuration

use std::{
    collections::VecDeque, fs::create_dir_all, net::SocketAddr, ops::Deref, path::PathBuf,
    sync::RwLockReadGuard,
};

use anyhow::{Context, Result};
use slug::slugify;
use tracing::{error, trace};

use ockam::identity::IdentityIdentifier;
pub use ockam_api::config::cli::NodeConfig;
use ockam_api::config::lookup::ProjectLookup;
pub use ockam_api::config::snippet::{
    ComposableSnippet, Operation, PortalMode, Protocol, RemoteMode,
};
use ockam_api::config::{cli, lookup::ConfigLookup, lookup::InternetAddress, Config};

use crate::util::exitcode;

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
    #[error("node with name {0} already exists")]
    Exists(String),
    #[error("node with name {0} does not exist")]
    NotFound(String),
    #[error("node with name {0} is not local")]
    NotLocal(String),
    #[error("provided value was not a valid {0}: {1}")]
    FailedConvert(String, String),
}

impl OckamConfig {
    pub fn load() -> Self {
        let directories = cli::OckamConfig::directories();
        let config_dir = directories.config_dir();
        let inner = Config::<cli::OckamConfig>::load(config_dir, "config");
        inner.writelock_inner().directories = Some(directories);
        Self { inner }
    }

    pub fn remove(self) -> Result<()> {
        let inner = self.inner.writelock_inner();
        let config_dir = inner
            .directories
            .as_ref()
            .context("configuration is in an invalid state")?
            .config_dir();
        if let Err(e) = std::fs::remove_dir_all(config_dir) {
            match e.kind() {
                std::io::ErrorKind::NotFound => {}
                _ => return Err(e.into()),
            }
        };
        let nodes_dir = inner
            .directories
            .as_ref()
            .context("configuration is in an invalid state")?
            .data_local_dir();
        if let Err(e) = std::fs::remove_dir_all(nodes_dir) {
            match e.kind() {
                std::io::ErrorKind::NotFound => {}
                _ => return Err(e.into()),
            }
        };
        Ok(())
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

    pub fn get_default_vault_path(&self) -> Option<PathBuf> {
        self.inner.readlock_inner().default_vault_path.clone()
    }

    pub fn get_default_identity(&self) -> Option<Vec<u8>> {
        self.inner.readlock_inner().default_identity.clone()
    }

    /// Get the node state directory
    pub fn get_node_dir(&self, name: &str) -> Result<PathBuf> {
        let inner = self.inner.readlock_inner();
        let n = inner
            .nodes
            .get(name)
            .ok_or_else(|| ConfigError::NotFound(name.to_string()))?;
        let node_path = n
            .state_dir
            .as_ref()
            .ok_or_else(|| ConfigError::NotLocal(name.to_string()))?;
        Ok(PathBuf::new().join(node_path))
    }

    /// Get the node state directory
    pub fn get_node_dir_raw(&self, name: &str) -> Result<PathBuf> {
        let dirs = cli::OckamConfig::directories();
        let nodes_dir = dirs.data_local_dir();
        let node_path = nodes_dir.join(slugify(&format!("node-{}", name)));
        Ok(node_path)
    }

    /// Get the API port used by a node
    pub fn get_node_port(&self, name: &str) -> u16 {
        let inner = self.inner.readlock_inner();
        inner
            .nodes
            .get(name)
            .unwrap_or_else(|| {
                eprintln!("No such node available. Run `ockam node list` to list available nodes");
                std::process::exit(exitcode::IOERR);
            })
            .port
    }

    /// In the future this will actually refer to the watchdog pid or
    /// no pid at all but we'll see
    pub fn get_node_pid(&self, name: &str) -> Result<Option<i32>> {
        let inner = self.inner.readlock_inner();
        Ok(inner
            .nodes
            .get(name)
            .ok_or_else(|| ConfigError::NotFound(name.to_string()))?
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
    pub fn get_node(&self, node: &str) -> Result<NodeConfig> {
        let inner = self.inner.readlock_inner();
        inner
            .nodes
            .get(node)
            .map(Clone::clone)
            .ok_or_else(|| ConfigError::NotFound(node.into()).into())
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
    pub fn log_paths_for_node(&self, node_name: &str) -> Option<(PathBuf, PathBuf)> {
        let inner = self.inner.readlock_inner();

        let base = inner.nodes.get(node_name)?.state_dir.as_ref()?;

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
    pub fn get_startup_cfg(&self, name: &str) -> Result<StartupConfig> {
        let path = self.get_node_dir(name)?;
        Ok(StartupConfig::load(path))
    }

    /// Get a lookup table for node alias -> internet address mappings
    pub fn get_lookup(&self) -> ConfigLookup {
        self.get_inner().get_lookup().clone()
    }

    pub fn authorities(&self, node: &str) -> Result<AuthoritiesConfig> {
        let path = self.get_node_dir_raw(node)?;
        Ok(AuthoritiesConfig::load(path))
    }

    ///////////////////// WRITE ACCESSORS //////////////////////////////

    pub fn set_default_vault_path(&self, default_vault_path: Option<PathBuf>) {
        self.inner.writelock_inner().default_vault_path = default_vault_path
    }

    pub fn set_default_identity(&self, default_identity: Option<Vec<u8>>) {
        self.inner.writelock_inner().default_identity = default_identity;
    }

    /// Add a new node to the configuration for future lookup
    pub fn create_node(&self, name: &str, bind: SocketAddr, verbose: u8) -> Result<()> {
        let mut inner = self.inner.writelock_inner();

        if inner.nodes.contains_key(name) {
            return Err(ConfigError::Exists(name.to_string()).into());
        }

        // Setup logging directory and store it
        let state_dir = inner
            .directories
            .as_ref()
            .context("configuration is in an invalid state")?
            .data_local_dir()
            .join(slugify(&format!("node-{}", name)));

        create_dir_all(&state_dir).context("failed to create new node state directory")?;

        // Add this node to the config lookup table
        inner.lookup.set_node(name, bind.into());

        // Set First Created Node as Default Node
        if inner.default.is_none() {
            inner.default = Some(name.to_string());
        }

        // Add this node to the main node table
        inner.nodes.insert(
            name.to_string(),
            NodeConfig {
                name: name.to_string(),
                port: bind.port(),
                addr: bind.into(),
                verbose,
                state_dir: Some(state_dir),
                pid: None,
            },
        );
        Ok(())
    }

    /// Delete an existing node
    pub fn delete_node(&self, name: &str) -> Result<()> {
        let mut inner = self.inner.writelock_inner();
        // If we are removing the first node also remove the default value
        match &inner.default {
            Some(default_node_name) if default_node_name == name => inner.default = None,
            _ => {}
        }
        inner.lookup.remove_node(name);
        match inner.nodes.remove(name) {
            Some(_) => Ok(()),
            None => Err(ConfigError::NotFound(name.to_string()).into()),
        }
    }

    /// Update the pid of an existing node process
    pub fn update_pid(&self, name: &str, pid: impl Into<Option<i32>>) -> Result<()> {
        let mut inner = self.inner.writelock_inner();

        if !inner.nodes.contains_key(name) {
            return Err(ConfigError::NotFound(name.to_string()).into());
        }

        inner.nodes.get_mut(name).unwrap().pid = pid.into();
        Ok(())
    }

    /// Update the api node name on record
    pub fn set_api_node(&self, node_name: &str) {
        let mut inner = self.inner.writelock_inner();
        inner.api_node = node_name.into();
    }

    pub fn set_node_alias(&self, alias: String, addr: InternetAddress) {
        let mut inner = self.inner.writelock_inner();
        inner.lookup.set_node(&alias, addr);
    }

    pub fn set_space_alias(&self, id: &str, name: &str) {
        let mut inner = self.inner.writelock_inner();
        inner.lookup.set_space(id, name);
        trace!(%id, %name, "Space stored in lookup table");
    }

    pub fn remove_space_alias(&self, name: &str) -> Result<()> {
        let mut inner = self.inner.writelock_inner();
        match inner.lookup.remove_space(name) {
            Some(_) => Ok(()),
            None => Err(ConfigError::Exists(name.to_string()).into()),
        }
    }

    pub fn remove_spaces_alias(&self) {
        let mut inner = self.inner.writelock_inner();
        inner.lookup.remove_spaces();
    }

    pub fn set_project_alias(&self, name: String, proj: ProjectLookup) -> Result<()> {
        let mut inner = self.inner.writelock_inner();
        trace! {
            id = %proj.id,
            name = %name,
            route = %proj.node_route,
            identity_id = %proj.identity_id,
            "Project stored in lookup table"
        };
        inner.lookup.set_project(name, proj);
        Ok(())
    }

    pub fn remove_project_alias(&self, name: &str) -> Result<()> {
        let mut inner = self.inner.writelock_inner();
        match inner.lookup.remove_project(name) {
            Some(_) => Ok(()),
            None => Err(ConfigError::Exists(name.to_string()).into()),
        }
    }

    pub fn remove_projects_alias(&self) {
        let mut inner = self.inner.writelock_inner();
        inner.lookup.remove_projects();
    }

    pub fn set_default_node(&self, name: &String) {
        let mut inner = self.inner.writelock_inner();
        inner.default = Some(name.to_string());
    }

    pub fn get_default_node(&self) -> Option<String> {
        let inner = self.inner.readlock_inner();
        inner.default.clone()
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
        let inner = Config::<cli::StartupConfig>::load(&node_dir, "startup");
        Self { inner }
    }

    /// Add a new composite command to a node
    pub fn add_composite(&self, composite: ComposableSnippet) {
        let mut inner = self.inner.writelock_inner();
        inner.commands.push_back(composite);
    }

    pub fn get_all(&self) -> VecDeque<ComposableSnippet> {
        self.inner.readlock_inner().commands.clone()
    }
}

#[derive(Debug)]
pub struct AuthoritiesConfig {
    inner: Config<cli::AuthoritiesConfig>,
}

impl AuthoritiesConfig {
    pub fn load(dir: PathBuf) -> Self {
        let inner = Config::<cli::AuthoritiesConfig>::load(&dir, "authorities");
        Self { inner }
    }

    pub fn add_authority(&self, i: IdentityIdentifier, a: cli::Authority) -> Result<()> {
        let mut cfg = self.inner.writelock_inner();
        cfg.add_authority(i, a);
        drop(cfg);
        self.inner.atomic_update().run()
    }

    pub fn snapshot(&self) -> cli::AuthoritiesConfig {
        self.inner.readlock_inner().clone()
    }
}
