use crate::node::CreateCommand;
use crate::run::parser::building_blocks::{as_command_args, ArgKey, ArgValue};
use crate::run::parser::resource::traits::ConfigRunner;
use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::{color_primary, node, Command, OckamSubcommand};
use async_trait::async_trait;
use miette::{miette, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub name: Option<ArgValue>,
    #[serde(alias = "skip-is-running-check")]
    pub skip_is_running_check: Option<ArgValue>,
    #[serde(alias = "exit-on-eof")]
    pub exit_on_eof: Option<ArgValue>,
    #[serde(alias = "tcp-listener-address")]
    pub tcp_listener_address: Option<ArgValue>,
    pub identity: Option<ArgValue>,
    pub project: Option<ArgValue>,
}

impl Node {
    pub const NAME_ARG: &'static str = "name";
}

#[async_trait]
impl ConfigRunner<CreateCommand> for Node {
    fn len(&self) -> usize {
        if self.name.is_some()
            || self.skip_is_running_check.is_some()
            || self.exit_on_eof.is_some()
            || self.tcp_listener_address.is_some()
            || self.identity.is_some()
            || self.project.is_some()
        {
            1
        } else {
            0
        }
    }

    fn into_commands(self) -> Result<Vec<CreateCommand>> {
        // Convert the struct into a map of key-value pairs
        let mut args: BTreeMap<ArgKey, ArgValue> = BTreeMap::new();
        if let Some(name) = self.name {
            args.insert("name".into(), name);
        }
        if let Some(skip_is_running_check) = self.skip_is_running_check {
            args.insert("skip-is-running-check".to_string(), skip_is_running_check);
        }
        if let Some(exit_on_eof) = self.exit_on_eof {
            args.insert("exit-on-eof".to_string(), exit_on_eof);
        }
        if let Some(tcp_listener_address) = self.tcp_listener_address {
            args.insert("tcp-listener-address".to_string(), tcp_listener_address);
        }
        if let Some(identity) = self.identity {
            args.insert("identity".to_string(), identity);
        }
        if let Some(project) = self.project {
            args.insert("project".to_string(), project);
        }
        if args.is_empty() {
            return Ok(vec![]);
        }

        // Convert the map into a list of cli args
        let mut cmd_args = vec![];
        // Remove "name" from the arguments and use it as a positional argument
        if let Some(name) = args.remove(Self::NAME_ARG) {
            cmd_args.push(name.to_string());
        }
        cmd_args.extend(as_command_args(args));
        Self::get_subcommand(&cmd_args).map(|c| vec![c])
    }

    fn get_subcommand(args: &[String]) -> Result<CreateCommand> {
        if let OckamSubcommand::Node(cmd) = parse_cmd_from_args(CreateCommand::NAME, args)? {
            if let node::NodeSubcommand::Create(c) = cmd.subcommand {
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
            let cmds = parsed.into_commands().unwrap();
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
