use miette::{miette, Result};
use ockam_api::colors::color_primary;
use serde::{Deserialize, Serialize};

use crate::run::parser::building_blocks::{ArgsToCommands, ResourceNameOrMap};

use crate::run::parser::resource::utils::parse_cmd_from_args;

use crate::tcp::outlet::create::CreateCommand;
use crate::{tcp::outlet, Command, OckamSubcommand};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TcpOutlets {
    #[serde(alias = "tcp-outlets", alias = "tcp-outlet")]
    pub tcp_outlets: Option<ResourceNameOrMap>,
}

impl TcpOutlets {
    fn get_subcommand(args: &[String]) -> Result<CreateCommand> {
        if let OckamSubcommand::TcpOutlet(cmd) = parse_cmd_from_args(CreateCommand::NAME, args)? {
            if let outlet::TcpOutletSubCommand::Create(c) = cmd.subcommand {
                return Ok(c);
            }
        }
        Err(miette!(format!(
            "Failed to parse {} command",
            color_primary(CreateCommand::NAME)
        )))
    }

    pub fn into_parsed_commands(
        self,
        default_node_name: Option<&String>,
    ) -> Result<Vec<CreateCommand>> {
        match self.tcp_outlets {
            Some(c) => {
                let mut cmds = c.into_commands_with_name_arg(Self::get_subcommand, Some("from"))?;
                if let Some(node_name) = default_node_name {
                    for cmd in cmds.iter_mut() {
                        if cmd.at.is_none() {
                            cmd.at = Some(node_name.to_string())
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
    use super::*;
    use ockam_node::HostnamePort;

    #[test]
    fn tcp_outlet_config() {
        let config = r#"
            tcp_outlets:
              to1:
                to: 6060
                at: n
              to2:
                to: 6061
                from: my_outlet
        "#;
        let parsed: TcpOutlets = serde_yaml::from_str(config).unwrap();
        let default_node_name = "n1".to_string();
        let cmds = parsed
            .into_parsed_commands(Some(&default_node_name))
            .unwrap();
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].from.clone().unwrap(), "to1");
        assert_eq!(cmds[0].to, HostnamePort::new("127.0.0.1", 6060));
        assert_eq!(cmds[0].at.as_ref().unwrap(), "n");
        assert_eq!(cmds[1].from.clone().unwrap(), "my_outlet");
        assert_eq!(cmds[1].to, HostnamePort::new("127.0.0.1", 6061));
        assert_eq!(cmds[1].at.as_ref(), Some(&default_node_name));
    }
}
