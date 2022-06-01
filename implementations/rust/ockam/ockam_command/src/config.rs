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

fn get_config_path() -> PathBuf {
    let proj = ProjectDirs::from("io", "ockam", "ockam-cli").expect(
        "failed to determine configuration storage location.
Verify that your XDG_CONFIG_HOME and XDG_DATA_HOME environment variables are correctly set.
Otherwise your OS or OS configuration may not be supported!",
    );

    let cfg_home = proj.config_dir();
    let _ = create_dir_all(&cfg_home);

    cfg_home.join("config.json")
}

impl OckamConfig {
    /// Attempt to load an ockam config.  If none exists, one is
    /// created and then returned.
    pub fn load() -> Self {
        let config_path = get_config_path();

        match File::open(&config_path) {
            Ok(ref mut f) => {
                let mut buf = String::new();
                f.read_to_string(&mut buf).expect("failed to read config");
                serde_json::from_str(&buf).expect("failed to parse config")
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

    /// Save the current config state
    pub fn save(&self) {
        let config_path = get_config_path();

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
    pub fn create_node(&mut self, name: &str, port: u16) -> Result<(), ConfigError> {
        if self.nodes.contains_key(name) {
            return Err(ConfigError::NodeExists(name.to_string()));
        }

        self.nodes.insert(name.to_string(), NodeConfig { port });
        Ok(())
    }

    /// Delete an existing node
    pub fn delete_node(&mut self, name: &str) -> Result<(), ConfigError> {
        match self.nodes.remove(name) {
            Some(_) => Ok(()),
            None => Err(ConfigError::NodeExists(name.to_string())),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct NodeConfig {
    port: u16,
}
