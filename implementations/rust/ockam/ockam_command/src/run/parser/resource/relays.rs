use miette::{miette, Result};
use ockam_api::colors::color_primary;
use serde::{Deserialize, Serialize};

use crate::relay::CreateCommand;
use crate::run::parser::building_blocks::{ArgsToCommands, ResourcesContainer};

use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::{relay, Command, OckamSubcommand};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relays {
    #[serde(alias = "relay")]
    pub relays: Option<ResourcesContainer>,
}

impl Relays {
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

    pub fn into_parsed_commands(
        self,
        default_node_name: &Option<String>,
    ) -> Result<Vec<CreateCommand>> {
        match self.relays {
            Some(c) => {
                let mut cmds = c.into_commands(Self::get_subcommand)?;
                if let Some(node_name) = default_node_name {
                    for cmd in cmds.iter_mut() {
                        if cmd.to.is_none() {
                            cmd.to = Some(node_name.clone());
                        }
                    }
                };
                Ok(cmds)
            }
            None => Ok(vec![]),
        }
    }
}

#[cfg(test)]
mod tests {
    use miette::IntoDiagnostic;

    use super::*;

    #[test]
    fn single_relay_config() {
        let test = |c: &str| {
            let parsed: Relays = serde_yaml::from_str(c).unwrap();
            let default_node_name = "n1".to_string();
            let cmds = parsed
                .into_parsed_commands(&Some(default_node_name.clone()))
                .unwrap();
            assert_eq!(cmds.len(), 1);
            let cmd = cmds.into_iter().next().unwrap();
            assert_eq!(cmd.relay_name, "r1");

            // if the 'to' value has not been explicitly set, use the default node name
            if cmd.to != Some("outlet-node".to_string()) {
                assert_eq!(cmd.to, Some(default_node_name));
            }
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
                to: outlet-node
        "#;
        test(config);
    }

    #[test]
    fn multiple_relay_config() {
        let test = |c: &str| {
            let parsed: Relays = serde_yaml::from_str(c).unwrap();
            let default_node_name = "n1".to_string();
            let cmds = parsed
                .into_parsed_commands(&Some(default_node_name.clone()))
                .unwrap();
            assert_eq!(cmds.len(), 2);
            assert_eq!(cmds[0].relay_name, "r1");
            assert_eq!(cmds[0].to, Some(default_node_name.clone()));

            assert_eq!(cmds[1].relay_name, "r2");
            assert_eq!(cmds[1].to, Some(default_node_name));
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
