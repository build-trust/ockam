use crate::config::{Config, ConfigValues};
pub use commands::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct NodeConfig {
    state: Config<NodeStateConfig>,
    commands: Config<Commands>,
}

impl NodeConfig {
    pub fn new(config_dir: &Path) -> anyhow::Result<Self> {
        let state = Config::load(config_dir, "state")?;
        let commands = Config::load(config_dir, "commands")?;
        Ok(Self { state, commands })
    }

    pub fn state(&self) -> &Config<NodeStateConfig> {
        &self.state
    }

    pub fn commands(&self) -> &Config<Commands> {
        &self.commands
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
    pub commands: Commands,
}

impl ConfigValues for NodeStateConfig {
    fn default_values(_config_dir: &Path) -> Self {
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
        fn default_values(_config_dir: &Path) -> Self {
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
