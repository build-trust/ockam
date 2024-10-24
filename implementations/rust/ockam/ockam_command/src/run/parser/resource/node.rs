use std::collections::BTreeMap;

use miette::{miette, Result};
use ockam_api::colors::color_primary;
use serde::{Deserialize, Serialize};

use crate::node::CreateCommand;
use crate::run::parser::building_blocks::{as_command_args, ArgKey, ArgValue};

use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::run::parser::resource::Resource;
use crate::{node, Command, OckamSubcommand};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Node {
    pub name: Option<ArgValue>,
    #[serde(alias = "skip-is-running-check")]
    pub skip_is_running_check: Option<ArgValue>,
    pub foreground: Option<ArgValue>,
    #[serde(alias = "child-process")]
    pub child_process: Option<ArgValue>,
    #[serde(alias = "exit-on-eof")]
    pub exit_on_eof: Option<ArgValue>,
    #[serde(alias = "tcp-listener-address")]
    pub tcp_listener_address: Option<ArgValue>,
    #[serde(alias = "http-server", alias = "enable-http-server")]
    pub http_server: Option<ArgValue>,
    #[serde(alias = "http-server-port")]
    pub http_server_port: Option<ArgValue>,
    pub identity: Option<ArgValue>,
    pub project: Option<ArgValue>,
    #[serde(alias = "launch-config")]
    pub launch_config: Option<ArgValue>,
    #[serde(alias = "opentelemetry-context")]
    pub opentelemetry_context: Option<ArgValue>,
}

impl Resource<CreateCommand> for Node {
    const COMMAND_NAME: &'static str = CreateCommand::NAME;

    fn args(self) -> Vec<String> {
        let mut args: BTreeMap<ArgKey, ArgValue> = BTreeMap::new();
        if let Some(name) = self.name {
            args.insert("name".into(), name);
        }
        if let Some(skip_is_running_check) = self.skip_is_running_check {
            args.insert("skip-is-running-check".into(), skip_is_running_check);
        }
        if let Some(foreground) = self.foreground {
            args.insert("foreground".into(), foreground);
        }
        if let Some(child_process) = self.child_process {
            args.insert("child-process".into(), child_process);
        }
        if let Some(exit_on_eof) = self.exit_on_eof {
            args.insert("exit-on-eof".into(), exit_on_eof);
        }
        if let Some(tcp_listener_address) = self.tcp_listener_address {
            args.insert("tcp-listener-address".into(), tcp_listener_address);
        }
        if let Some(enable_http_server) = self.http_server {
            args.insert("http-server".into(), enable_http_server);
        }
        if let Some(http_server_port) = self.http_server_port {
            args.insert("http-server-port".into(), http_server_port);
        }
        if let Some(identity) = self.identity {
            args.insert("identity".into(), identity);
        }
        if let Some(project) = self.project {
            args.insert("project".into(), project);
        }
        if let Some(launch_config) = self.launch_config {
            args.insert("launch-config".into(), launch_config);
        }
        if let Some(opentelemetry_context) = self.opentelemetry_context {
            args.insert("opentelemetry-context".into(), opentelemetry_context);
        }
        if args.is_empty() {
            return vec![];
        }

        // Convert the map into a list of cli args
        let mut cmd_args = vec![];
        // Remove "name" from the arguments and use it as a positional argument
        if let Some(ArgValue::String(name)) = args.remove(&Self::NAME_ARG.into()) {
            cmd_args.push(name.to_string());
        }
        cmd_args.extend(as_command_args(args));
        cmd_args
    }
}

impl Node {
    pub const NAME_ARG: &'static str = "name";

    /// Return the node name if defined
    pub fn name(&self) -> Option<String> {
        self.name
            .clone()
            .map(|v| match v {
                ArgValue::String(s) => Some(s),
                _ => None,
            })
            .unwrap_or(None)
    }

    /// Return the identity name if defined
    pub fn identity(&self) -> Option<String> {
        self.identity
            .clone()
            .map(|v| match v {
                ArgValue::String(s) => Some(s),
                _ => None,
            })
            .unwrap_or(None)
    }

    pub fn into_parsed_commands(self) -> Result<Vec<CreateCommand>> {
        Ok(vec![Self::get_subcommand(&self.args())?])
    }

    fn get_subcommand(args: &[String]) -> Result<CreateCommand> {
        if let OckamSubcommand::Node(cmd) = parse_cmd_from_args(CreateCommand::NAME, args)? {
            if let node::NodeSubcommand::Create(mut c) = cmd.subcommand {
                c.config_args.configuration = None;
                c.config_args.variables = vec![];
                c.config_args.enrollment_ticket = None;
                return Ok(c);
            }
        }
        Err(miette!(format!(
            "Failed to parse {} command",
            color_primary(CreateCommand::NAME)
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_config() {
        let test = |c: &str| {
            let parsed: Node = serde_yaml::from_str(c).unwrap();
            let cmds = parsed.into_parsed_commands().unwrap();
            assert_eq!(cmds.len(), 1);
            let cmd = cmds.into_iter().next().unwrap();
            assert_eq!(cmd.name, "n1");
        };

        // Name only
        let config = r#"
            name: n1
        "#;
        test(config);

        // Multiple arguments
        let config = r#"
            name: n1
            tcp-listener-address: 127.0.0.1:1234
            skip-is-running-check: true
        "#;
        test(config);

        // With other sections
        let config = r#"
            relays: r1

            name: n1
            tcp-listener-address: 127.0.0.1:1234
            skip-is-running-check: true

            tcp_inlets:
              ti1:
                from: 6060
                at: n
        "#;
        test(config);
    }
}
