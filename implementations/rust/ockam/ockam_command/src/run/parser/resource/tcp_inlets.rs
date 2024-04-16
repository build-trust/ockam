use miette::{miette, Result};
use ockam_api::colors::color_primary;
use serde::{Deserialize, Serialize};

use crate::run::parser::building_blocks::{ArgsToCommands, ResourceNameOrMap};

use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::tcp::inlet::create::CreateCommand;
use crate::{tcp::inlet, Command, OckamSubcommand};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TcpInlets {
    #[serde(alias = "tcp-inlets", alias = "tcp-inlet")]
    pub tcp_inlets: Option<ResourceNameOrMap>,
}

impl TcpInlets {
    fn get_subcommand(args: &[String]) -> Result<CreateCommand> {
        if let OckamSubcommand::TcpInlet(cmd) = parse_cmd_from_args(CreateCommand::NAME, args)? {
            if let inlet::TcpInletSubCommand::Create(c) = cmd.subcommand {
                return Ok(c);
            }
        }
        Err(miette!(format!(
            "Failed to parse {} command",
            color_primary(CreateCommand::NAME)
        )))
    }

    pub fn parse_commands(self, default_node_name: &Option<String>) -> Result<Vec<CreateCommand>> {
        match self.tcp_inlets {
            Some(c) => {
                let mut cmds =
                    c.into_commands_with_name_arg(Self::get_subcommand, Some("alias"))?;
                if let Some(node_name) = default_node_name.as_ref() {
                    for cmd in cmds.iter_mut() {
                        if cmd.at.is_none() {
                            cmd.at = Some(node_name.clone())
                        }
                    }
                }
                Ok(cmds)
            }
            None => Ok(vec![]),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::str::FromStr;

    use super::*;

    #[test]
    fn tcp_inlet_config() {
        let named = r#"
            tcp_inlets:
              ti1:
                from: 6060
                at: n
              ti2:
                from: '6061'
                alias: my_inlet
        "#;
        let parsed: TcpInlets = serde_yaml::from_str(named).unwrap();
        let default_node_name = "n1".to_string();
        let cmds = parsed
            .parse_commands(&Some(default_node_name.clone()))
            .unwrap();
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].alias, "ti1");
        assert_eq!(
            cmds[0].from,
            SocketAddr::from_str("127.0.0.1:6060").unwrap()
        );
        assert_eq!(cmds[0].at.as_ref().unwrap(), "n");
        assert_eq!(cmds[1].alias, "my_inlet");
        assert_eq!(
            cmds[1].from,
            SocketAddr::from_str("127.0.0.1:6061").unwrap()
        );
        assert_eq!(cmds[1].at, Some(default_node_name.clone()));

        let unnamed = r#"
            tcp_inlets:
              - from: 6060
                at: n
              - from: '6061'
        "#;
        let parsed: TcpInlets = serde_yaml::from_str(unnamed).unwrap();
        let cmds = parsed
            .parse_commands(&Some(default_node_name.clone()))
            .unwrap();
        assert_eq!(cmds.len(), 2);
        assert_eq!(
            cmds[0].from,
            SocketAddr::from_str("127.0.0.1:6060").unwrap()
        );
        assert_eq!(cmds[0].at.as_ref().unwrap(), "n");
        assert_eq!(
            cmds[1].from,
            SocketAddr::from_str("127.0.0.1:6061").unwrap()
        );
        assert_eq!(cmds[1].at, Some(default_node_name));
    }
}
