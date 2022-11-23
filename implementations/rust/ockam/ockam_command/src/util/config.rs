//! Handle local node configuration

use std::{fs::create_dir_all, net::SocketAddr, ops::Deref, path::PathBuf, sync::RwLockReadGuard};

use anyhow::{Context, Result};
use tracing::{error, trace};

use ockam::identity::IdentityIdentifier;
use ockam_api::config::cli::NodeConfigOld;
use ockam_api::config::lookup::ProjectLookup;
use ockam_api::config::{cli, lookup::ConfigLookup, lookup::InternetAddress, Config};
use ockam_api::nodes::config::NodeConfig;

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
    AlreadyExists(String),
    #[error("node with name {0} does not exist")]
    NotFound(String),
    #[error("node with name {0} is not local")]
    NotLocal(String),
}

impl OckamConfig {
    pub fn load() -> Result<OckamConfig> {
        let dir = cli::OckamConfig::dir();
        let inner = Config::<cli::OckamConfig>::load(&dir, "config")?;
        inner.write().dir = Some(dir);
        Ok(Self { inner })
    }

    pub fn node(&self, name: &str) -> Result<NodeConfig> {
        let dir = cli::OckamConfig::node_dir(name);
        if !dir.exists() {
            return Err(ConfigError::NotFound(name.to_string()).into());
        }
        NodeConfig::new(&dir)
    }

    pub fn remove(self) -> Result<()> {
        let inner = self.inner.write();
        // Try to delete CLI directory. If the directory is not found,
        // we do nothing. Otherwise, we return the error.
        let dir = inner
            .dir
            .as_ref()
            .context("configuration is in an invalid state")?;
        if let Err(e) = std::fs::remove_dir_all(dir) {
            match e.kind() {
                std::io::ErrorKind::NotFound => {}
                _ => return Err(e.into()),
            }
        };
        Ok(())
    }

    pub fn get_default_vault_path(&self) -> Option<PathBuf> {
        self.inner.read().default_vault_path.clone()
    }

    pub fn get_default_identity(&self) -> Option<Vec<u8>> {
        self.inner.read().default_identity.clone()
    }

    /// Get the node state directory
    pub fn get_node_dir(&self, name: &str) -> Result<PathBuf> {
        let inner = self.inner.read();
        let n = inner
            .nodes
            .get(name)
            .ok_or_else(|| ConfigError::NotFound(name.to_string()))?;
        let node_path = n
            .state_dir()
            .ok_or_else(|| ConfigError::NotLocal(name.to_string()))?;
        Ok(PathBuf::new().join(node_path))
    }

    /// Get the node state directory
    pub fn get_node_dir_unchecked(&self, name: &str) -> PathBuf {
        cli::OckamConfig::node_dir(name)
    }

    /// Get the API port used by a node
    pub fn get_node_port(&self, name: &str) -> Result<u16> {
        let inner = self.inner.read();
        let port = inner
            .nodes
            .get(name)
            .context("No such node available. Run `ockam node list` to list available nodes")?
            .port();

        Ok(port)
    }

    /// In the future this will actually refer to the watchdog pid or
    /// no pid at all but we'll see
    pub fn get_node_pid(&self, name: &str) -> Result<Option<i32>> {
        let inner = self.inner.read();
        Ok(inner
            .nodes
            .get(name)
            .ok_or_else(|| ConfigError::NotFound(name.to_string()))?
            .pid())
    }

    /// Check whether another node has been registered with this API
    /// port.  This doesn't catch all port collision errors, but will
    /// get us most of the way there in terms of starting a new node.
    pub fn port_is_used(&self, port: u16) -> bool {
        let inner = self.inner.read();
        inner.nodes.iter().any(|(_, n)| n.port() == port)
    }

    /// Get only a single node configuration
    pub fn get_node(&self, node: &str) -> Result<NodeConfigOld> {
        let inner = self.inner.read();
        inner
            .nodes
            .get(node)
            .map(Clone::clone)
            .ok_or_else(|| ConfigError::NotFound(node.into()).into())
    }

    /// Get the current version the selected node configuration
    pub fn select_node<'a>(&'a self, o: &'a str) -> Option<NodeConfigOld> {
        let inner = self.inner.read();
        inner.nodes.get(o).map(Clone::clone)
    }

    /// Get the log path for a specific node
    ///
    /// The convention is to name the main log `stdout.log` and the
    /// supplementary log `stderr.log`
    pub fn node_log_paths(&self, node_name: &str) -> Option<(PathBuf, PathBuf)> {
        let inner = self.inner.read();
        let base = inner.nodes.get(node_name)?.state_dir()?;
        Some((base.join("stdout.log"), base.join("stderr.log")))
    }

    /// Get read access to the inner raw configuration
    pub fn inner(&self) -> RwLockReadGuard<'_, cli::OckamConfig> {
        self.inner.read()
    }

    /// Get a lookup table
    pub fn lookup(&self) -> ConfigLookup {
        self.inner().lookup().clone()
    }

    pub fn authorities(&self, node: &str) -> Result<AuthoritiesConfig> {
        let path = self.get_node_dir_unchecked(node);
        AuthoritiesConfig::load(path)
    }

    ///////////////////// WRITE ACCESSORS //////////////////////////////

    pub fn set_default_vault_path(&self, default_vault_path: Option<PathBuf>) {
        self.inner.write().default_vault_path = default_vault_path
    }

    pub fn set_default_identity(&self, default_identity: Option<Vec<u8>>) {
        self.inner.write().default_identity = default_identity;
    }

    /// Add a new node to the configuration for future lookup
    pub fn create_node(&self, name: &str, bind: SocketAddr, verbose: u8) -> Result<()> {
        let mut inner = self.inner.write();

        if inner.nodes.contains_key(name) {
            return Err(ConfigError::AlreadyExists(name.to_string()).into());
        }

        // Create node's state directory
        let dir = cli::OckamConfig::node_dir(name);
        create_dir_all(&dir).context("failed to create new node state directory")?;

        // Initialize it
        NodeConfig::init_for_new_node(&dir)?;

        // Add this node to the config lookup table
        inner.lookup.set_node(name, bind.into());

        // Set First Created Node as Default Node
        if inner.default.is_none() {
            inner.default = Some(name.to_string());
        }

        // Add this node to the main node table
        inner.nodes.insert(
            name.to_string(),
            NodeConfigOld::new(
                name.to_string(),
                bind.into(),
                bind.port(),
                verbose,
                None,
                Some(dir),
            ),
        );
        Ok(())
    }

    /// Delete an existing node
    ///
    /// Since this is an idempotent operation and there could be multiple nodes performing the same
    /// deletion operation, we don't return an error if the node doesn't exist.
    pub fn remove_node(&self, name: &str) {
        let mut inner = self.inner.write();
        // If we are removing the first node also remove the default value
        match &inner.default {
            Some(default_node_name) if default_node_name == name => inner.default = None,
            _ => {}
        }
        inner.lookup.remove_node(name);
        inner.nodes.remove(name);
    }

    /// Update the pid of an existing node process
    pub fn set_node_pid(&self, name: &str, pid: impl Into<Option<i32>>) -> Result<()> {
        let mut inner = self.inner.write();

        if !inner.nodes.contains_key(name) {
            return Err(ConfigError::NotFound(name.to_string()).into());
        }

        inner.nodes.get_mut(name).unwrap().pid = pid.into();
        Ok(())
    }

    pub fn set_node_alias(&self, alias: String, addr: InternetAddress) {
        let mut inner = self.inner.write();
        inner.lookup.set_node(&alias, addr);
    }

    pub fn set_space_alias(&self, id: &str, name: &str) {
        let mut inner = self.inner.write();
        inner.lookup.set_space(id, name);
    }

    pub fn remove_space_alias(&self, name: &str) {
        let mut inner = self.inner.write();
        inner.lookup.remove_space(name);
    }

    pub fn remove_spaces_alias(&self) {
        let mut inner = self.inner.write();
        inner.lookup.remove_spaces();
    }

    pub fn set_project_alias(&self, name: String, proj: ProjectLookup) -> Result<()> {
        let mut inner = self.inner.write();
        trace! {
            id = %proj.id,
            name = %name,
            route = ?proj.node_route,
            identity_id = ?proj.identity_id,
            "Project stored in lookup table"
        };
        inner.lookup.set_project(name, proj);
        Ok(())
    }

    pub fn remove_project_alias(&self, name: &str) {
        let mut inner = self.inner.write();
        inner.lookup.remove_project(name);
    }

    pub fn remove_projects_alias(&self) {
        let mut inner = self.inner.write();
        inner.lookup.remove_projects();
    }

    pub fn set_default_node(&self, name: &String) {
        let mut inner = self.inner.write();
        inner.default = Some(name.to_string());
    }

    pub fn get_default_node(&self) -> Option<String> {
        let inner = self.inner.read();
        inner.default.clone()
    }
}

#[derive(Debug)]
pub struct AuthoritiesConfig {
    inner: Config<cli::AuthoritiesConfig>,
}

impl AuthoritiesConfig {
    pub fn load(dir: PathBuf) -> Result<Self> {
        let inner = Config::<cli::AuthoritiesConfig>::load(&dir, "authorities")?;
        Ok(Self { inner })
    }

    pub fn add_authority(&self, i: IdentityIdentifier, a: cli::Authority) -> Result<()> {
        let mut cfg = self.inner.write();
        cfg.add_authority(i, a);
        drop(cfg);
        self.inner.persist_config_updates()
    }

    pub fn snapshot(&self) -> cli::AuthoritiesConfig {
        self.inner.read().clone()
    }
}
