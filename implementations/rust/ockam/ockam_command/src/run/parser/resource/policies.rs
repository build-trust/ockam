use crate::policy::CreateCommand;
use crate::run::parser::building_blocks::{ArgsToCommands, UnnamedResources};
use crate::run::parser::resource::traits::ConfigRunner;
use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::{color_primary, policy, Command, OckamSubcommand};
use async_trait::async_trait;
use miette::{miette, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Policies {
    #[serde(alias = "policy")]
    pub policies: Option<UnnamedResources>,
}

#[async_trait]
impl ConfigRunner<CreateCommand> for Policies {
    fn len(&self) -> usize {
        match &self.policies {
            Some(c) => c.len(),
            None => 0,
        }
    }

    fn into_commands(self) -> Result<Vec<CreateCommand>> {
        match self.policies {
            Some(c) => c.into_commands(Self::get_subcommand),
            None => Ok(vec![]),
        }
    }

    fn get_subcommand(args: &[String]) -> Result<CreateCommand> {
        if let OckamSubcommand::Policy(cmd) = parse_cmd_from_args(CreateCommand::NAME, args)? {
            if let policy::PolicySubcommand::Create(c) = cmd.subcommand {
                return Ok(c);
            }
        }
        Err(miette!(format!(
            "Failed to parse {} command",
            color_primary(CreateCommand::NAME)
        )))
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
