use crate::identity::CreateCommand;
use crate::run::parser::building_blocks::{ArgsToCommands, ResourcesContainer};
use crate::run::parser::resource::traits::CommandsParser;
use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::{color_primary, identity, Command, OckamSubcommand};
use async_trait::async_trait;
use miette::{miette, Result};

use crate::run::parser::resource::ValuesOverrides;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Identities {
    #[serde(alias = "identity")]
    pub identities: Option<ResourcesContainer>,
}

impl Identities {
    fn get_subcommand(args: &[String]) -> Result<CreateCommand> {
        if let OckamSubcommand::Identity(cmd) = parse_cmd_from_args(CreateCommand::NAME, args)? {
            if let identity::IdentitySubcommand::Create(c) = cmd.subcommand {
                return Ok(c);
            }
        }
        Err(miette!(format!(
            "Failed to parse {} command",
            color_primary(CreateCommand::NAME)
        )))
    }
}

#[async_trait]
impl CommandsParser<CreateCommand> for Identities {
    fn parse_commands(self, _overrides: &ValuesOverrides) -> Result<Vec<CreateCommand>> {
        match self.identities {
            Some(c) => c.into_commands(Self::get_subcommand),
            None => Ok(vec![]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use miette::IntoDiagnostic;

    #[test]
    fn single_identity_config() {
        let test = |c: &str| {
            let parsed: Identities = serde_yaml::from_str(c).unwrap();
            let cmds = parsed.parse_commands(&ValuesOverrides::default()).unwrap();
            assert_eq!(cmds.len(), 1);
            let cmd = cmds.into_iter().next().unwrap();
            assert_eq!(cmd.name, "i1");
        };

        // Name only
        let config = r#"
            identities:
              - i1
        "#;
        test(config);

        let config = r#"
            identities: i1
        "#;
        test(config);

        // Config only
        let config = r#"
            identities:
              i1:
                vault: v1
        "#;
        test(config);
    }

    #[test]
    fn multiple_identity_config() {
        let test = |c: &str| {
            let parsed: Identities = serde_yaml::from_str(c).unwrap();
            let cmds = parsed.parse_commands(&ValuesOverrides::default()).unwrap();
            assert_eq!(cmds.len(), 2);
            assert_eq!(cmds[0].name, "i1");
            assert_eq!(cmds[1].name, "i2");
        };

        // Name only
        let config = r#"
            identities:
              - i1
              - i2
        "#;
        test(config);

        // Config only
        let config = r#"
            identities:
              i1:
                vault: v1
              i2:
                vault: v1
        "#;
        test(config);

        // Mixing name and args as a list
        let config = r#"
            identities:
              - i1
              - i2:
                  vault: v1
        "#;
        test(config);

        // Mixing name and args as a map will fail
        let config = r#"
            identities:
              i1:
              i2:
                vault: v1
        "#;
        let parsed: Result<Identities> = serde_yaml::from_str(config).into_diagnostic();
        assert!(parsed.is_err());
    }
}
