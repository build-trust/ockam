use crate::relay::CreateCommand;
use crate::run::parser::building_blocks::{ArgsToCommands, ResourcesContainer};
use crate::run::parser::resource::traits::ConfigRunner;
use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::run::parser::resource::PreRunHooks;
use crate::{color_primary, relay, Command, CommandGlobalOpts, OckamSubcommand};
use async_trait::async_trait;
use miette::{miette, Result};
use ockam_node::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relays {
    #[serde(alias = "relay")]
    pub relays: Option<ResourcesContainer>,
}

#[async_trait]
impl ConfigRunner<CreateCommand> for Relays {
    fn len(&self) -> usize {
        match &self.relays {
            Some(c) => c.len(),
            None => 0,
        }
    }

    fn into_commands(self) -> Result<Vec<CreateCommand>> {
        match self.relays {
            Some(c) => c.into_commands(Self::get_subcommand),
            None => Ok(vec![]),
        }
    }

    fn get_subcommand(args: &[String]) -> Result<CreateCommand> {
        if let OckamSubcommand::Relay(cmd) = parse_cmd_from_args(CreateCommand::NAME, args)? {
            if let relay::RelaySubCommand::Create(c) = cmd.subcommand {
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
            cmd.to = Some(node_name);
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use miette::IntoDiagnostic;

    #[test]
    fn single_relay_config() {
        let test = |c: &str| {
            let parsed: Relays = serde_yaml::from_str(c).unwrap();
            let cmds = parsed.into_commands().unwrap();
            assert_eq!(cmds.len(), 1);
            let cmd = cmds.into_iter().next().unwrap();
            assert_eq!(cmd.relay_name, "r1");
        };

        // Name only
        let config = r#"
            relays:
              - r1
        "#;
        test(config);

        let config = r#"
            relays: r1
        "#;
        test(config);

        // Config only
        let config = r#"
            relays:
              r1:
                at: /project/default
        "#;
        test(config);
    }

    #[test]
    fn multiple_relay_config() {
        let test = |c: &str| {
            let parsed: Relays = serde_yaml::from_str(c).unwrap();
            let cmds = parsed.into_commands().unwrap();
            assert_eq!(cmds.len(), 2);
            assert_eq!(cmds[0].relay_name, "r1");
            assert_eq!(cmds[1].relay_name, "r2");
        };

        // Name only
        let config = r#"
            relays:
              - r1
              - r2
        "#;
        test(config);

        // Config only
        let config = r#"
            relays:
              r1:
                at: /project/default
              r2:
                at: /project/default
        "#;
        test(config);

        // Mixing name and args as a list
        let config = r#"
            relays:
              - r1
              - r2:
                  at: /project/default
        "#;
        test(config);

        // Mixing name and args as a map will fail
        let config = r#"
            relays:
              r1:
              r2:
                at: /project/default
        "#;
        let parsed: Result<Relays> = serde_yaml::from_str(config).into_diagnostic();
        assert!(parsed.is_err());
    }
}
