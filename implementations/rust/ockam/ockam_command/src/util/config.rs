//! Handle local node configuration

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{create_dir_all, File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Default, Serialize, Deserialize)]
pub struct OckamConfig {
    pub log_path: PathBuf,
    pub api_node: String,
    nodes: BTreeMap<String, NodeConfig>,
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

fn get_paths() -> (PathBuf, PathBuf) {
    let proj = ProjectDirs::from("io", "ockam", "ockam-cli").expect(
        "failed to determine configuration storage location.
Verify that your XDG_CONFIG_HOME and XDG_DATA_HOME environment variables are correctly set.
Otherwise your OS or OS configuration may not be supported!",
    );

    let cfg_home = proj.config_dir();
    let _ = create_dir_all(&cfg_home);

    let data_home = proj.data_local_dir();
    let _ = create_dir_all(&data_home);

    (cfg_home.join("config.json"), data_home.to_path_buf())
}

impl OckamConfig {
    /// Return a static set of config values that can be addressed
    pub fn values() -> Vec<&'static str> {
        vec!["api-node", "log-path"]
    }

    fn new(log_path: PathBuf) -> Self {
        Self {
            log_path,
            api_node: "default".into(),
            ..Default::default()
        }
    }

    /// Attempt to load an ockam config.  If none exists, one is
    /// created and then returned.
    pub fn load() -> Self {
        let (config_path, log_path) = get_paths();

        match File::open(&config_path) {
            Ok(ref mut f) => {
                let mut buf = String::new();
                f.read_to_string(&mut buf).expect("failed to read config");
                serde_json::from_str(&buf).expect("failed to parse config")
            }
            Err(_) => {
                let new = Self::new(log_path);
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

    /// Save the current config state
    pub fn save(&self) {
        let (config_path, _) = get_paths();

        let mut file = OpenOptions::new()
            .create(false)
            .truncate(true)
            .write(true)
            .open(config_path)
            .expect("failed to open config for writing");

        let json: String = serde_json::to_string_pretty(self).expect("failed to serialise config");
        file.write_all(json.as_bytes())
            .expect("failed to write config");
    }

    /// Add a new node to the configuration for future lookup
    pub fn create_node(&mut self, name: &str, port: u16, pid: i32) -> Result<(), ConfigError> {
        if self.nodes.contains_key(name) {
            return Err(ConfigError::NodeExists(name.to_string()));
        }

        self.nodes.insert(
            name.to_string(),
            NodeConfig {
                port,
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
        self.nodes.iter().find(|(_, n)| n.port == port).is_some()
    }

    /// Get read-acces to all node configuration
    pub fn get_nodes(&self) -> &BTreeMap<String, NodeConfig> {
        &self.nodes
    }

    /// Get the log path for a specific node
    pub fn log_path(&self, node_name: &String) -> String {
        self.log_path
            .join(format!("{}.log", node_name))
            .to_str()
            .unwrap()
            .to_owned()
    }

    /// Update the api node name on record
    pub fn set_api_node(&mut self, node_name: &String) {
        self.api_node = node_name.clone();
    }

    /// Update the base log path for nodes
    pub fn set_log_path(&mut self, path: &String) {
        self.log_path = PathBuf::new().join(path);
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeConfig {
    pub port: u16,
    pub pid: Option<i32>,
}
