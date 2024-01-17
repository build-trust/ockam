use crate::run::parser::{parse_cmd_from_args, ArgsToCommands, ResourcesNamesAndArgs};
use crate::tcp::outlet::create::CreateCommand;
use crate::{tcp::outlet, OckamSubcommand};
use miette::{miette, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TcpOutlets {
    #[serde(alias = "tcp-outlets")]
    pub tcp_outlets: Option<ResourcesNamesAndArgs>,
}

impl ArgsToCommands<CreateCommand> for TcpOutlets {
    fn into_commands(self) -> Result<Vec<CreateCommand>> {
        let get_subcommand = |args: &[String]| -> Result<CreateCommand> {
            if let OckamSubcommand::TcpOutlet(cmd) = parse_cmd_from_args("tcp-outlet create", args)?
            {
                if let outlet::TcpOutletSubCommand::Create(c) = cmd.subcommand {
                    return Ok(c);
                }
            }
            Err(miette!("Failed to parse command"))
        };
        match self.tcp_outlets {
            Some(c) => c.into_commands(get_subcommand, Some("alias")),
            None => Ok(vec![]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use std::str::FromStr;

    #[test]
    fn tcp_outlet_config() {
        let config = r#"
            tcp_outlets:
              to1:
                to: '6060'
                at: n
              to2:
                to: '6061'
        "#;
        let parsed: TcpOutlets = serde_yaml::from_str(config).unwrap();
        let cmds = parsed.into_commands().unwrap();
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].alias.as_ref().unwrap(), "to1");
        assert_eq!(cmds[0].to, SocketAddr::from_str("127.0.0.1:6060").unwrap());
        assert_eq!(cmds[0].at.as_ref().unwrap(), "n");
        assert_eq!(cmds[1].alias.as_ref().unwrap(), "to2");
        assert_eq!(cmds[1].to, SocketAddr::from_str("127.0.0.1:6061").unwrap());
        assert!(cmds[1].at.is_none());
    }
}
