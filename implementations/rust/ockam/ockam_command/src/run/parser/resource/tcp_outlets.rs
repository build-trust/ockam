use crate::run::parser::building_blocks::{ArgsToCommands, ResourceNameOrMap};
use crate::run::parser::resource::traits::ConfigRunner;
use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::run::parser::resource::PreRunHooks;
use crate::tcp::outlet::create::CreateCommand;
use crate::{color_primary, tcp::outlet, Command, CommandGlobalOpts, OckamSubcommand};
use async_trait::async_trait;
use miette::{miette, Result};
use ockam_node::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TcpOutlets {
    #[serde(alias = "tcp-outlets", alias = "tcp-outlet")]
    pub tcp_outlets: Option<ResourceNameOrMap>,
}

#[async_trait]
impl ConfigRunner<CreateCommand> for TcpOutlets {
    fn len(&self) -> usize {
        match &self.tcp_outlets {
            Some(c) => c.len(),
            None => 0,
        }
    }

    fn into_commands(self) -> Result<Vec<CreateCommand>> {
        match self.tcp_outlets {
            Some(c) => c.into_commands_with_name_arg(Self::get_subcommand, Some("from")),
            None => Ok(vec![]),
        }
    }

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

    async fn pre_run_hooks(
        _ctx: &Context,
        _opts: &CommandGlobalOpts,
        hooks: &PreRunHooks,
        cmd: &mut CreateCommand,
    ) -> Result<bool> {
        if let Some(node_name) = hooks.override_node_name.clone() {
            cmd.at = Some(node_name);
        }
        Ok(true)
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
                to: 6060
                at: n
              to2:
                to: 6061
                from: my_outlet
        "#;
        let parsed: TcpOutlets = serde_yaml::from_str(config).unwrap();
        let cmds = parsed.into_commands().unwrap();
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].from.clone().unwrap(), "to1");
        assert_eq!(cmds[0].to, SocketAddr::from_str("127.0.0.1:6060").unwrap());
        assert_eq!(cmds[0].at.as_ref().unwrap(), "n");
        assert_eq!(cmds[1].from.clone().unwrap(), "my_outlet");
        assert_eq!(cmds[1].to, SocketAddr::from_str("127.0.0.1:6061").unwrap());
        assert!(cmds[1].at.is_none());
    }
}
