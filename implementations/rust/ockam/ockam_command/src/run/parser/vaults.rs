use crate::run::parser::resources::{parse_cmd_from_args, ArgsToCommands, ResourcesContainer};
use crate::vault::CreateCommand;
use crate::{vault, OckamSubcommand};
use miette::{miette, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Vaults {
    pub vaults: Option<ResourcesContainer>,
}

impl ArgsToCommands<CreateCommand> for Vaults {
    fn into_commands(self) -> Result<Vec<CreateCommand>> {
        let get_subcommand = |args: &[String]| -> Result<CreateCommand> {
            if let OckamSubcommand::Vault(cmd) = parse_cmd_from_args("vault create", args)? {
                if let vault::VaultSubcommand::Create(c) = cmd.subcommand {
                    return Ok(c);
                }
            }
            Err(miette!("Failed to parse command"))
        };
        match self.vaults {
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
    fn single_vault_config() {
        let test = |c: &str| {
            let parsed: Vaults = serde_yaml::from_str(c).unwrap();
            let cmds = parsed.into_commands().unwrap();
            assert_eq!(cmds.len(), 1);
            let cmd = cmds.into_iter().next().unwrap();
            assert_eq!(cmd.name.as_ref().unwrap(), "v1");
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
            let cmds = parsed.into_commands().unwrap();
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
