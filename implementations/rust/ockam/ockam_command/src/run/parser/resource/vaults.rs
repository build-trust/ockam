use crate::run::parser::building_blocks::{ArgsToCommands, ResourcesContainer};
use crate::run::parser::resource::traits::CommandsParser;
use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::vault::CreateCommand;
use crate::{color_primary, vault, Command, OckamSubcommand};
use async_trait::async_trait;
use miette::{miette, Result};

use crate::run::parser::resource::ValuesOverrides;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vaults {
    #[serde(alias = "vault")]
    pub vaults: Option<ResourcesContainer>,
}

impl Vaults {
    fn get_subcommand(args: &[String]) -> Result<CreateCommand> {
        if let OckamSubcommand::Vault(cmd) = parse_cmd_from_args(CreateCommand::NAME, args)? {
            if let vault::VaultSubcommand::Create(c) = cmd.subcommand {
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
impl CommandsParser<CreateCommand> for Vaults {
    fn parse_commands(self, _overrides: &ValuesOverrides) -> Result<Vec<CreateCommand>> {
        match self.vaults {
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
    fn single_vault_config() {
        let test = |c: &str| {
            let parsed: Vaults = serde_yaml::from_str(c).unwrap();
            let cmds = parsed.parse_commands(&ValuesOverrides::default()).unwrap();
            assert_eq!(cmds.len(), 1);
            assert_eq!(cmds[0].name.as_ref().unwrap(), "v1");
        };

        // Name only
        let config = r#"
            vaults:
              - v1
        "#;
        test(config);

        let config = r#"
            vaults: v1
        "#;
        test(config);

        // Config only
        let config = r#"
            vaults:
              v1:
                path: ./vault.path
                aws-kms: false
        "#;
        test(config);
    }

    #[test]
    fn multiple_vaults_config() {
        let test = |c: &str| {
            let parsed: Vaults = serde_yaml::from_str(c).unwrap();
            let cmds = parsed.parse_commands(&ValuesOverrides::default()).unwrap();
            assert_eq!(cmds.len(), 2);
            assert_eq!(cmds[0].name.as_ref().unwrap(), "v1");
            assert_eq!(cmds[1].name.as_ref().unwrap(), "v2");
        };

        // Name only
        let config = r#"
            vaults:
              - v1
              - v2
        "#;
        test(config);

        // Config only
        let config = r#"
            vaults:
              v1:
                aws-kms: true
              v2:
                path: ./vault.path
        "#;
        test(config);

        // Mixing name and args as a list
        let config = r#"
            vaults:
              - v1
              - v2:
                  aws-kms: true
        "#;
        test(config);

        // Mixing name and args as a map will fail
        let config = r#"
            vaults:
              v1:
              v2:
                aws-kms: true
        "#;
        let parsed: Result<Vaults> = serde_yaml::from_str(config).into_diagnostic();
        assert!(parsed.is_err());
    }
}
