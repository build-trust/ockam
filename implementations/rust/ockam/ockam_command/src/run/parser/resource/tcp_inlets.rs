use crate::run::parser::building_blocks::{ArgsToCommands, ResourceNameOrMap};
use crate::run::parser::resource::traits::ConfigRunner;
use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::run::parser::resource::PreRunHooks;
use crate::tcp::inlet::create::CreateCommand;
use crate::{color_primary, tcp::inlet, Command, CommandGlobalOpts, OckamSubcommand};
use async_trait::async_trait;
use miette::{miette, Result};
use ockam_node::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TcpInlets {
    #[serde(alias = "tcp-inlets", alias = "tcp-inlet")]
    pub tcp_inlets: Option<ResourceNameOrMap>,
}

#[async_trait]
impl ConfigRunner<CreateCommand> for TcpInlets {
    fn len(&self) -> usize {
        match &self.tcp_inlets {
            Some(c) => c.len(),
            None => 0,
        }
    }

    fn into_commands(self) -> Result<Vec<CreateCommand>> {
        match self.tcp_inlets {
            Some(c) => c.into_commands_with_name_arg(Self::get_subcommand, Some("alias")),
            None => Ok(vec![]),
        }
    }

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
        let cmds = parsed.into_commands().unwrap();
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
        assert!(cmds[1].at.is_none());

        let unnamed = r#"
            tcp_inlets:
              - from: 6060
                at: n
              - from: '6061'
        "#;
        let parsed: TcpInlets = serde_yaml::from_str(unnamed).unwrap();
        let cmds = parsed.into_commands().unwrap();
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
        assert!(cmds[1].at.is_none());
    }
}
