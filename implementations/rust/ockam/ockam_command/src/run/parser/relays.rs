use crate::relay::CreateCommand;
use crate::run::parser::{parse_cmd_from_args, ArgsToCommands, ResourcesContainer};
use crate::{relay, OckamSubcommand};
use miette::{miette, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Relays {
    pub relays: Option<ResourcesContainer>,
}

impl ArgsToCommands<CreateCommand> for Relays {
    fn into_commands(self) -> Result<Vec<CreateCommand>> {
        let get_subcommand = |args: &[String]| -> Result<CreateCommand> {
            if let OckamSubcommand::Relay(cmd) = parse_cmd_from_args("relay create", args)? {
                if let relay::RelaySubCommand::Create(c) = cmd.subcommand {
                    return Ok(c);
                }
            }
            Err(miette!("Failed to parse command"))
        };
        match self.relays {
            Some(c) => c.into_commands(get_subcommand),
            None => Ok(vec![]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use miette::IntoDiagnostic;

    #[test]
    fn test_single_relay_config() {
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
    fn test_multiple_relay_config() {
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

        // Mixing name and config will fail
        let config = r#"
            relays:
              - r1
              r2:
                at: /project/default
        "#;
        let parsed: Result<Relays> = serde_yaml::from_str(config).into_diagnostic();
        assert!(parsed.is_err());
    }
}
