use std::fmt::Formatter;
use std::io::Write;
use std::{
    fmt::Display,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

pub use commands::*;

use crate::config::{build_config_path, Config, ConfigValues};

#[derive(Debug)]
pub struct NodeConfig {
    state: Config<NodeStateConfig>,
    commands: Config<Commands>,
}

impl NodeConfig {
    pub fn new(config_dir: &Path) -> anyhow::Result<Self> {
        let version = NodeConfigVersion::load(config_dir)?;
        let state = match version.state_config_name() {
            None => Config::default(),
            Some(path) => Config::load(config_dir, path)?,
        };
        let commands = match version.commands_config_name() {
            None => Config::default(),
            Some(path) => Config::load(config_dir, path)?,
        };
        Ok(Self { state, commands })
    }

    pub fn init_for_new_node(config_dir: &Path) -> anyhow::Result<()> {
        let v = NodeConfigVersion::latest();
        for dir in v.dirs() {
            let path = config_dir.join(dir);
            if path.exists() {
                return Err(anyhow!("Config file already exists: {}", path.display()));
            }
        }
        NodeConfigVersion::set_version(config_dir, &v)?;
        Ok(())
    }

    pub fn state(&self) -> &Config<NodeStateConfig> {
        &self.state
    }

    pub fn commands(&self) -> &Config<Commands> {
        &self.commands
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NodeConfigVersion {
    V0,
    V1,
}

impl NodeConfigVersion {
    const FILE_NAME: &'static str = "version";

    fn latest() -> Self {
        Self::V1
    }

    fn load(config_dir: &Path) -> anyhow::Result<Self> {
        let version_path = config_dir.join(Self::FILE_NAME);
        let version = if version_path.exists() {
            let mut version_file = File::open(version_path)?;
            let mut version = String::new();
            version_file.read_to_string(&mut version)?;
            NodeConfigVersion::from_str(&version)?
        } else {
            Self::V0
        };
        debug!(%version, "Loaded config");
        version.upgrade(config_dir)
    }

    fn upgrade(&self, config_dir: &Path) -> anyhow::Result<Self> {
        let from = self;
        let mut final_version = from.clone();

        // Iter through all the versions between `from` and `to`
        let f = from.to_string().parse::<u8>()?;
        let mut t = f + 1;
        while let Ok(ref to) = Self::from_str(&t.to_string()) {
            debug!(%from, %to, "Upgrading config");
            final_version = to.clone();
            #[allow(clippy::single_match)]
            match (from, to) {
                (Self::V0, Self::V1) => {
                    if let (Some(old_config_name), Some(new_config_name)) =
                        (from.state_config_name(), to.state_config_name())
                    {
                        let old_config_path = build_config_path(config_dir, old_config_name);
                        // If old config path exists, copy to new config path and keep the old one
                        if old_config_path.exists() {
                            let new_config_path = build_config_path(config_dir, new_config_name);
                            std::fs::copy(old_config_path, new_config_path)?;
                        }
                        // Create the version file if doesn't exists
                        Self::set_version(config_dir, to)?;
                    }
                }
                _ => {}
            }
            t += 1;
        }
        Ok(final_version)
    }

    fn dirs(&self) -> &'static [&'static str] {
        match self {
            Self::V0 => &["config"],
            Self::V1 => &["state", "commands"],
        }
    }

    fn state_config_name(&self) -> Option<&'static str> {
        match self {
            Self::V0 => Some("config"),
            Self::V1 => Some("state"),
        }
    }

    fn commands_config_name(&self) -> Option<&'static str> {
        match self {
            Self::V0 => None,
            Self::V1 => Some("commands"),
        }
    }

    fn set_version(config_dir: &Path, version: &NodeConfigVersion) -> anyhow::Result<()> {
        let version_path = config_dir.join(Self::FILE_NAME);
        let mut version_file = File::create(version_path)?;
        version_file.write_all(version.to_string().as_bytes())?;
        Ok(())
    }
}

impl Display for NodeConfigVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            NodeConfigVersion::V0 => "0",
            NodeConfigVersion::V1 => "1",
        })
    }
}

impl FromStr for NodeConfigVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(Self::V0),
            "1" => Ok(Self::V1),
            _ => Err(anyhow!("Unknown version: {}", s)),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct NodeStateConfig {
    /// Lmdb file location
    pub authenticated_storage_path: Option<PathBuf>,
    /// Vault info
    pub vault_path: Option<PathBuf>,
    /// Exported identity value
    pub identity: Option<Vec<u8>>,
    /// Identity was overridden
    pub identity_was_overridden: bool,
}

impl ConfigValues for NodeStateConfig {
    fn default_values() -> Self {
        Self::default()
    }
}

mod commands {
    use super::*;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
    #[cfg_attr(test, derive(Eq, PartialEq))]
    pub struct Commands {
        /// Run when executing the `node run` command.
        pub run: Option<RunNode>,
        /// Run when the node is created.
        #[serde(default)]
        pub on_node_init: Vec<Command>,
        /// Run after initialization is done. Will rerun on every node restart.
        #[serde(default)]
        pub on_node_startup: Vec<Command>,
    }

    impl ConfigValues for Commands {
        fn default_values() -> Self {
            Self::default()
        }
    }

    impl Config<Commands> {
        pub fn set(&self, cmds: Commands) -> anyhow::Result<()> {
            {
                let mut inner = self.write();
                inner.run = cmds.run;
                inner.on_node_init = cmds.on_node_init;
                inner.on_node_startup = cmds.on_node_startup;
            }
            self.persist_config_updates()?;
            Ok(())
        }
    }

    impl Command {
        pub fn new(command: String, pipe: Option<bool>) -> Self {
            if pipe.is_none() {
                Command::String(command)
            } else {
                Command::Obj(CommandObj {
                    command: Some(command),
                    args: None,
                    pipe,
                })
            }
        }

        /// Return the `args` field as a Vec of strings.
        pub fn args(&self) -> Vec<String> {
            match self {
                Command::String(s) => s.split_whitespace().map(|s| s.to_string()).collect(),
                Command::Obj(o) => o.args(),
            }
        }

        pub fn pipe_output(&self) -> bool {
            match self {
                Command::String(_) => false,
                Command::Obj(o) => o.pipe_output(),
            }
        }
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
    #[cfg_attr(test, derive(Eq, PartialEq))]
    pub struct RunNode {
        pub name: String,
        pub args: Option<CommandArgs>,
    }

    impl RunNode {
        /// Return the `args` field as a Vec of strings.
        pub fn args<P: AsRef<Path>>(&self, path: P, exe: &str) -> Vec<String> {
            let mut args: Vec<String> = format!("{} node create {}", exe, self.name)
                .split_whitespace()
                .map(|x| x.to_string())
                .collect();
            if let Some(a) = &self.args {
                args.extend(a.args());
            }
            args.extend([
                "--config".to_string(),
                path.as_ref().to_str().expect("Invalid path").to_string(),
            ]);
            args
        }
    }

    impl From<RunNode> for Command {
        fn from(r: RunNode) -> Self {
            let args = match r.args {
                Some(args) => match args {
                    CommandArgs::String(s) => s,
                    CommandArgs::Vec(v) => v.join(" "),
                },
                None => String::new(),
            };
            Command::String(format!("node create {} {}", r.name, args))
        }
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(untagged)]
    #[cfg_attr(test, derive(Eq, PartialEq))]
    pub enum Command {
        String(String),
        Obj(CommandObj),
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[cfg_attr(test, derive(Eq, PartialEq))]
    pub struct CommandObj {
        pub command: Option<String>,
        pub args: Option<CommandArgs>,
        /// Pipes output to the next command.
        pub pipe: Option<bool>,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
    #[serde(untagged)]
    pub enum CommandArgs {
        String(String),
        Vec(Vec<String>),
    }

    impl CommandArgs {
        pub fn args(&self) -> Vec<String> {
            match self {
                CommandArgs::String(s) => s.split_whitespace().map(|s| s.to_string()).collect(),
                // `join` and `split` to convert to single-value entries.
                // E.g. ["--flag", "--arg value"] -> ["--flag", "--arg", "value"]
                CommandArgs::Vec(v) => v
                    .join(" ")
                    .split(' ')
                    .map(|s| s.to_string())
                    .filter(|x| !x.is_empty())
                    .collect(),
            }
        }
    }

    impl CommandObj {
        /// Return the `args` field as a Vec of strings.
        fn args(&self) -> Vec<String> {
            let mut args = Vec::new();
            // Collect args from `command` and `args`
            if let Some(command) = &self.command {
                args.extend(command.split_whitespace().map(|s| s.to_string()));
            }
            if let Some(a) = &self.args {
                args.extend(a.args());
            }
            // Add `--pipe` if needed. This can be used by commands to output a different message based on this flag.
            if self.pipe_output() {
                args.push("--pipe".to_string());
            }
            args
        }

        fn pipe_output(&self) -> bool {
            self.pipe.unwrap_or(false)
        }
    }
}
