use crate::identity::CreateCommand;
use crate::run::parser::resources::{parse_cmd_from_args, ArgsToCommands, ResourcesContainer};
use crate::{identity, OckamSubcommand};
use miette::{miette, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Identities {
    pub identities: Option<ResourcesContainer>,
}

impl ArgsToCommands<CreateCommand> for Identities {
    fn into_commands(self) -> Result<Vec<CreateCommand>> {
        let get_subcommand = |args: &[String]| -> Result<CreateCommand> {
            if let OckamSubcommand::Identity(cmd) = parse_cmd_from_args("identity create", args)? {
                if let identity::IdentitySubcommand::Create(c) = cmd.subcommand {
                    return Ok(c);
                }
            }
            Err(miette!("Failed to parse command"))
        };
        match self.identities {
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
    fn single_identity_config() {
        let test = |c: &str| {
            let parsed: Identities = serde_yaml::from_str(c).unwrap();
            let cmds = parsed.into_commands().unwrap();
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
            let cmds = parsed.into_commands().unwrap();
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
