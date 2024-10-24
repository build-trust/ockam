use miette::{miette, Result};
use ockam_api::colors::color_primary;
use serde::{Deserialize, Serialize};

use crate::node::CreateCommand;
use crate::run::parser::building_blocks::{ArgsToCommands, ResourcesContainer};

use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::{node, Command, OckamSubcommand};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Nodes {
    #[serde(alias = "node")]
    pub nodes: Option<ResourcesContainer>,
}

impl Nodes {
    fn get_subcommand(args: &[String]) -> Result<CreateCommand> {
        if let OckamSubcommand::Node(cmd) = parse_cmd_from_args(CreateCommand::NAME, args)? {
            if let node::NodeSubcommand::Create(c) = cmd.subcommand {
                return Ok(c);
            }
        }
        Err(miette!(format!(
            "Failed to parse {} command",
            color_primary(CreateCommand::NAME)
        )))
    }

    pub fn into_parsed_commands(self) -> Result<Vec<CreateCommand>> {
        match self.nodes {
            Some(c) => c.into_commands(Self::get_subcommand),
            None => Ok(vec![]),
        }
    }
}

#[cfg(test)]
mod tests {
    use miette::IntoDiagnostic;

    use super::*;

    #[test]
    fn single_node_config() {
        let test = |c: &str| {
            let parsed: Nodes = serde_yaml::from_str(c).unwrap();
            let cmds = parsed.into_parsed_commands().unwrap();
            assert_eq!(cmds.len(), 1);
            let cmd = cmds.into_iter().next().unwrap();
            assert_eq!(cmd.name, "n1");
        };

        // Name only
        let config = r#"
            nodes:
              - n1
        "#;
        test(config);

        let config = r#"
            nodes: n1
        "#;
        test(config);

        // Config only
        let config = r#"
            nodes:
              n1:
                identity: i1
        "#;
        test(config);
    }

    #[test]
    fn multiple_node_config() {
        let test = |c: &str| {
            let parsed: Nodes = serde_yaml::from_str(c).unwrap();
            let cmds = parsed.into_parsed_commands().unwrap();
            assert_eq!(cmds.len(), 2);
            assert_eq!(cmds[0].name, "n1");
            assert_eq!(cmds[1].name, "n2");
        };

        // Name only
        let config = r#"
            nodes:
              - n1
              - n2
        "#;
        test(config);

        // Config only
        let config = r#"
            nodes:
              n1:
                identity: i1
              n2:
                project: p1
        "#;
        test(config);

        // Mixing name and args will fail
        let config = r#"
            nodes:
              - n1
              n2:
                identity: i1
        "#;
        let parsed: Result<Nodes> = serde_yaml::from_str(config).into_diagnostic();
        assert!(parsed.is_err());
    }
}
