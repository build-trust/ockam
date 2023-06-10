use crate::Result;
use duct::Expression;
use ockam_api::cli_state::{CliState, StateDirTrait};
use ockam_core::compat::collections::HashMap;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::fmt::Debug;
use std::path::Path;
use tracing::debug;

pub struct ConfigRunner {
    commands_sorted: Vec<ParsedCommand>,
    commands_index: BTreeMap<String, usize>,
}

#[derive(Clone)]
pub struct ParsedCommand {
    pub id: String,
    pub depends_on: Option<String>,
    pub cmd: Expression,
}

impl ConfigRunner {
    fn new() -> Self {
        Self {
            commands_sorted: vec![],
            commands_index: Default::default(),
        }
    }

    pub fn go(cli_state: &CliState, path: &Path) -> Result<()> {
        let mut cr = Self::new();
        cr.parse(cli_state, path)?;
        cr.run()?;
        Ok(())
    }

    fn parse(&mut self, cli_state: &CliState, path: &Path) -> Result<()> {
        let config = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&config)?;
        let mut visited = HashSet::new();
        let mut nodes = VecDeque::new();
        for (name, node) in config.nodes {
            nodes.push_back((name, node));
        }
        while let Some((name, node)) = nodes.pop_front() {
            // If the node depends on another node, check if that node has been parsed.
            if let Some(depends_on) = &node.depends_on {
                // If the dependency has been visited already but not
                // parsed, we have a circular dependency.
                if visited.contains(depends_on) {
                    return Err(miette::miette!(
                        "Circular dependency detected: {} -> {}",
                        depends_on,
                        name
                    )
                    .into());
                }
                // If the dependency has been parsed, remove it from the control
                // vector and proceed with the current node.
                if self
                    .commands_index
                    .contains_key(&format!("node/{depends_on}"))
                {
                    visited.remove(depends_on);
                }
                // If the dependency has not been parsed, push the current
                // node back to the queue and continue with the next one.
                if !self
                    .commands_index
                    .contains_key(&format!("node/{depends_on}"))
                {
                    visited.insert(name.clone());
                    nodes.push_back((name, node));
                    continue;
                }
            }
            // Remove it from the control vector and parse it.
            visited.remove(&name);
            node.parse(cli_state, &name, self)?;
        }
        Ok(())
    }

    fn run(self) -> Result<()> {
        for c in self.commands_sorted.into_iter() {
            debug!("Running command: {}", c.id);
            // If a command fails it will show the appropriate error in its subshell.
            // No need to return an error here.
            if c.cmd.run().is_err() {
                break;
            }
        }
        Ok(())
    }
}

/// The config structure will be a yml file with the following structure:
/// ```yml
/// nodes:
///   telegraf:
///     enrollment-token: $OCKAM_TELEGRAF_TOKEN
///     tcp-inlets:
///       telegraf:
///         from: '127.0.0.1:8087'
///         to: /project/default/service/forward_to_influxdb/secure/api/service/outlet
///         access_control: '(= subject.component "influxdb")'
///
///   influxdb:
///     enrollment-token: $OCKAM_INFLUXDB_TOKEN
///     tcp-outlets:
///       influxdb:
///         from: /service/outlet
///         to: '127.0.0.1:8086'
///         access_control: '(= subject.component "telegraf")'
///     forwarders:
///       influxdb:
///         at: /project/default
/// ```
#[derive(Debug, Deserialize)]
pub struct Config {
    pub nodes: HashMap<String, NodeConfig>,
}

/// Defines the structure of a node in the config file.
#[derive(Debug, Deserialize)]
pub struct NodeConfig {
    #[serde(rename(deserialize = "depends-on"))]
    pub depends_on: Option<String>,
    pub enrollment_token: Option<String>,
    #[serde(rename(deserialize = "tcp-inlets"))]
    pub tcp_inlets: Option<HashMap<String, InletConfig>>,
    #[serde(rename(deserialize = "tcp-outlets"))]
    pub tcp_outlets: Option<HashMap<String, OutletConfig>>,
    pub forwarders: Option<HashMap<String, ForwarderConfig>>,
}

impl NodeConfig {
    fn parse(self, cli_state: &CliState, node_name: &str, cmds: &mut ConfigRunner) -> Result<()> {
        let mut insert_command = |subject: &str, name: &str, depends_on, args: &[&str]| {
            debug!("Parsed command: {} {}", binary_path(), args.join(" "));
            let cmd = duct::cmd(binary_path(), args);
            let id = format!("{subject}/{name}");
            if cmds.commands_index.contains_key(&id) {
                return Err(miette::miette!(
                    "There can't be {}s with the same name: {}",
                    subject,
                    name
                ));
            }
            cmds.commands_index
                .insert(id.clone(), cmds.commands_sorted.len());
            cmds.commands_sorted.push(ParsedCommand {
                id,
                depends_on,
                cmd,
            });
            Ok(())
        };

        // Check if the node already exists. If it doesn't, create it.
        if !cli_state.nodes.exists(node_name) {
            let args = {
                let mut args = vec!["node", "create", node_name];
                if let Some(enrollment_token) = &self.enrollment_token {
                    args.push("--enrollment-token");
                    args.push(enrollment_token);
                }
                args
            };
            insert_command(
                "node",
                node_name,
                self.depends_on.map(|s| format!("node/{s}")),
                args.as_slice(),
            )?;
        }

        // TODO: all commands should support both `/node/{name}` and `{name}` formats.
        let node_name_formatted = format!("/node/{node_name}");

        if let Some(tcp_inlets) = &self.tcp_inlets {
            for (name, inlet) in tcp_inlets {
                // TODO: store inlets in CliState; Then check if the inlet already exists. If it doesn't, create it.
                if let Some(exp) = &inlet.access_control {
                    let args = &[
                        "policy",
                        "create",
                        "--at",
                        &node_name_formatted,
                        "--resource",
                        "tcp-inlet",
                        "--expression",
                        exp,
                    ];
                    insert_command("policy", name, None, args)?;
                }
                let args = &[
                    "tcp-inlet",
                    "create",
                    "--at",
                    &node_name_formatted,
                    "--from",
                    &inlet.from,
                    "--to",
                    &inlet.to,
                    "--alias",
                    name,
                ];
                insert_command("inlet", name, None, args)?;
            }
        }

        if let Some(tcp_outlets) = &self.tcp_outlets {
            for (name, outlet) in tcp_outlets {
                // TODO: store outlets in CliState; Then check if the outlet already exists. If it doesn't, create it.
                if let Some(exp) = &outlet.access_control {
                    let args = &[
                        "policy",
                        "create",
                        "--at",
                        &node_name_formatted,
                        "--resource",
                        "tcp-outlet",
                        "--expression",
                        exp,
                    ];
                    insert_command("policy", name, None, args)?;
                }
                let args = &[
                    "tcp-outlet",
                    "create",
                    "--at",
                    &node_name_formatted,
                    "--from",
                    &outlet.from,
                    "--to",
                    &outlet.to,
                    "--alias",
                    name,
                ];
                insert_command("outlet", name, None, args)?;
            }
        }

        if let Some(forwarders) = &self.forwarders {
            for (name, forwarder) in forwarders {
                // TODO: store forwarders in CliState; Then check if the forwarder already exists. If it doesn't, create it.
                let args = &[
                    "forwarder",
                    "create",
                    name,
                    "--to",
                    &node_name_formatted,
                    "--at",
                    &forwarder.at,
                ];
                insert_command("forwarder", name, None, args)?;
            }
        }

        Ok(())
    }
}

/// Defines the structure of a tcp inlet in the config file.
#[derive(Debug, Deserialize)]
pub struct InletConfig {
    pub from: String,
    pub to: String,
    pub access_control: Option<String>,
}

/// Defines the structure of a tcp outlet in the config file.
#[derive(Debug, Deserialize)]
pub struct OutletConfig {
    pub from: String,
    pub to: String,
    pub access_control: Option<String>,
}

/// Defines the structure of a forwarder in the config file.
#[derive(Debug, Deserialize)]
pub struct ForwarderConfig {
    pub at: String,
}

static BINARY_PATH: Lazy<String> = Lazy::new(|| {
    std::env::args()
        .next()
        .expect("Failed to get the binary path")
});

fn binary_path() -> &'static str {
    &BINARY_PATH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config_with_depends_on() {
        let config = r#"
            nodes:
              influxdb:
                tcp-outlets:
                  influxdb:
                    from: /service/outlet
                    to: '127.0.0.1:8086'
                    access_control: '(= subject.component "telegraf")'
                forwarders:
                  influxdb:
                    at: /project/default

              telegraf:
                depends-on: influxdb
                tcp-inlets:
                  telegraf:
                    from: '127.0.0.1:8087'
                    to: /project/default/service/forward_to_influxdb/secure/api/service/outlet
                    access_control: '(= subject.component "influxdb")'
        "#;
        let tmp_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp_file.path(), config).unwrap();

        let mut sut = ConfigRunner::new();
        let cli_state = CliState::test().unwrap();
        sut.parse(&cli_state, tmp_file.path()).unwrap();

        assert_eq!(sut.commands_sorted.len(), 7);
        assert_eq!(sut.commands_sorted[0].id, "node/influxdb");
        assert_eq!(sut.commands_sorted[1].id, "policy/influxdb");
        assert_eq!(sut.commands_sorted[2].id, "outlet/influxdb");
        assert_eq!(sut.commands_sorted[3].id, "forwarder/influxdb");
        assert_eq!(sut.commands_sorted[4].id, "node/telegraf");
        assert_eq!(
            sut.commands_sorted[4].depends_on.as_ref().unwrap(),
            "node/influxdb"
        );
        assert_eq!(sut.commands_sorted[5].id, "policy/telegraf");
        assert_eq!(sut.commands_sorted[6].id, "inlet/telegraf");
    }

    #[test]
    fn detect_circular_dependency() {
        let cases = vec![
            (
                r#"
                    nodes:
                      node1:
                        depends-on: node2
                      node2:
                        depends-on: node1
                "#,
                Err(()),
            ),
            (
                r#"
                    nodes:
                      node1:
                        depends-on: node2
                      node2:
                        depends-on: node3
                      node3:
                        depends-on: node1
                "#,
                Err(()),
            ),
            (
                r#"
                    nodes:
                      node1:
                        depends-on: node2
                      node2:
                        depends-on: node1
                      node3:
                "#,
                Err(()),
            ),
            (
                r#"
                    nodes:
                      node1:
                        depends-on: node3
                      node2:
                        depends-on: node3
                      node3:
                "#,
                Ok(()),
            ),
            (
                r#"
                    nodes:
                      node1:
                      node2:
                "#,
                Ok(()),
            ),
        ];
        let tmp_file = tempfile::NamedTempFile::new().unwrap();
        for (config, expected) in cases {
            std::fs::write(tmp_file.path(), config).unwrap();
            let mut sut = ConfigRunner::new();
            let cli_state = CliState::test().unwrap();
            let result = sut.parse(&cli_state, tmp_file.path());
            match expected {
                Ok(_) => assert!(result.is_ok()),
                Err(_) => {
                    assert!(result.is_err());
                    assert!(result
                        .unwrap_err()
                        .to_string()
                        .contains("Circular dependency detected"));
                }
            }
        }
    }
}
