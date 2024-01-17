use crate::node::CreateCommand;
use crate::run::parser::{parse_cmd_from_args, ArgsToCommands, ResourcesContainer};
use crate::{node, OckamSubcommand};
use miette::{miette, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Nodes {
    pub nodes: Option<ResourcesContainer>,
}

impl ArgsToCommands<CreateCommand> for Nodes {
    fn into_commands(self) -> Result<Vec<CreateCommand>> {
        let get_subcommand = |args: &[String]| -> Result<CreateCommand> {
            if let OckamSubcommand::Node(cmd) = parse_cmd_from_args("node create", args)? {
                if let node::NodeSubcommand::Create(c) = cmd.subcommand {
                    return Ok(c);
                }
            }
            Err(miette!("Failed to parse command"))
        };
        match self.nodes {
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
    fn test_single_node_config() {
        let test = |c: &str| {
            let parsed: Nodes = serde_yaml::from_str(c).unwrap();
            let cmds = parsed.into_commands().unwrap();
            assert_eq!(cmds.len(), 1);
            let cmd = cmds.into_iter().next().unwrap();
            assert_eq!(cmd.node_name, "n1");
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
    fn test_multiple_node_config() {
        let test = |c: &str| {
            let parsed: Nodes = serde_yaml::from_str(c).unwrap();
            let cmds = parsed.into_commands().unwrap();
            assert_eq!(cmds.len(), 2);
            assert_eq!(cmds[0].node_name, "n1");
            assert_eq!(cmds[1].node_name, "n2");
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

        // Mixing name and config will fail
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
