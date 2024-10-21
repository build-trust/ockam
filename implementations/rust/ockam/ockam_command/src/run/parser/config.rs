use miette::IntoDiagnostic;
use ockam_core::errcode::{Kind, Origin};
use serde::Deserialize;

use crate::run::parser::Variables;

pub struct ConfigParser;

impl ConfigParser {
    pub fn parse<'de, T: Deserialize<'de>>(contents: &'de mut String) -> miette::Result<T> {
        // Resolve the environment variables section
        Variables::resolve(contents)?;

        // Parse the configuration file as the given T type
        serde_yaml::from_str(contents)
            .map_err(|e| {
                ockam_core::Error::new(
                    Origin::Api,
                    Kind::Serialization,
                    format!(
                        "could not parse the configuration file: {e:?}\n\n{}",
                        contents
                    ),
                )
            })
            .into_diagnostic()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::parser::building_blocks::ArgsToCommands;
    use crate::run::parser::resource::{Nodes, Relays};
    use serde::Serialize;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    pub struct TestConfig {
        #[serde(flatten)]
        pub nodes: Nodes,
        #[serde(flatten)]
        pub relays: Relays,
    }

    #[test]
    fn parse_yaml_config() {
        let mut contents = r#"
            variables:
              var_b: true
              var_i: 1 # comment
            nodes:
              # comment
              - node1
              - node2
            relays:
              r1:
                at: /project/default
                to: outlet-node
        "#
        .to_string();

        let result = ConfigParser::parse::<TestConfig>(&mut contents).unwrap();
        assert_eq!(std::env::var("var_b").unwrap(), "true");
        assert_eq!(std::env::var("var_i").unwrap(), "1");
        assert_eq!(result.nodes.nodes.unwrap().len(), 2);
    }

    #[test]
    fn parse_json_config() {
        let mut contents = r#"
            {
                # comment
                "variables": {
                    "var_b": true,
                    "var_i": 1, # trailing commas are supported
                },
                "nodes": [
                    "node1",
                    "node2",
                ],
                "relays": {
                    "r1": {
                        # comment
                        "at": "/project/default",
                        "to": "outlet-node",
                    }
                }
            }
        "#
        .to_string();
        let result = ConfigParser::parse::<TestConfig>(&mut contents).unwrap();
        assert_eq!(std::env::var("var_b").unwrap(), "true");
        assert_eq!(std::env::var("var_i").unwrap(), "1");
        assert_eq!(result.nodes.nodes.unwrap().len(), 2);
    }
}
