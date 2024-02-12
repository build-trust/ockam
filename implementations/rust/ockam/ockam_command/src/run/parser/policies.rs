use crate::policy::CreateCommand;
use crate::run::parser::resources::{parse_cmd_from_args, ArgsToCommands, UnnamedResources};
use crate::{policy, OckamSubcommand};
use miette::{miette, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Policies {
    pub policies: Option<UnnamedResources>,
}

impl ArgsToCommands<CreateCommand> for Policies {
    fn into_commands(self) -> Result<Vec<CreateCommand>> {
        let get_subcommand = |args: &[String]| -> Result<CreateCommand> {
            if let OckamSubcommand::Policy(cmd) = parse_cmd_from_args("policy create", args)? {
                if let policy::PolicySubcommand::Create(c) = cmd.subcommand {
                    return Ok(c);
                }
            }
            Err(miette!("Failed to parse command"))
        };
        match self.policies {
            Some(c) => c.into_commands(get_subcommand),
            None => Ok(vec![]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_policy_config() {
        let config = r#"
            policies:
              at: n1
              resource: r1
              expression: (= subject.component "c1")
        "#;
        let parsed: Policies = serde_yaml::from_str(config).unwrap();
        let cmds = parsed.into_commands().unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].at.as_ref().unwrap(), "n1");
        assert_eq!(cmds[0].resource.as_ref().unwrap().as_str(), "r1");
        assert_eq!(
            &cmds[0].expression.to_string(),
            "(= subject.component \"c1\")"
        );
    }

    #[test]
    fn multiple_policy_config() {
        let config = r#"
            policies:
              - at: n1
                resource: r1
                expression: (= subject.component "c1")
              - at: n2
                resource: tcp-outlet
                expression: (= subject.component "c2")
              - at: n3
                resource-type: tcp-inlet
                expression: (= subject.component "c3")
        "#;
        let parsed: Policies = serde_yaml::from_str(config).unwrap();
        let cmds = parsed.into_commands().unwrap();
        assert_eq!(cmds.len(), 3);

        assert_eq!(cmds[0].at.as_ref().unwrap(), "n1");
        assert_eq!(cmds[0].resource.as_ref().unwrap().as_str(), "r1");
        assert_eq!(
            &cmds[0].expression.to_string(),
            "(= subject.component \"c1\")"
        );

        assert_eq!(cmds[1].at.as_ref().unwrap(), "n2");
        assert_eq!(
            &cmds[1].resource.as_ref().unwrap().to_string(),
            "tcp-outlet"
        );
        assert_eq!(
            &cmds[1].expression.to_string(),
            "(= subject.component \"c2\")"
        );

        assert_eq!(cmds[2].at.as_ref().unwrap(), "n3");
        assert_eq!(
            &cmds[2].resource_type.as_ref().unwrap().to_string(),
            "tcp-inlet"
        );
        assert_eq!(
            &cmds[2].expression.to_string(),
            "(= subject.component \"c3\")"
        );
    }
}
