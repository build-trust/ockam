use miette::IntoDiagnostic;
use ockam_core::errcode::{Kind, Origin};
use serde::Deserialize;

use crate::run::parser::Variables;

pub struct ConfigParser;

impl ConfigParser {
    pub fn parse<'de, T: Deserialize<'de>>(contents: &'de mut String) -> miette::Result<T> {
        // Expand the environment variables section
        Variables::expand(contents)?;

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
    use crate::run::parser::resource::{Nodes, Relays};
    use serde::Serialize;
    use serial_test::serial;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    pub struct TestConfig {
        #[serde(flatten)]
        pub nodes: Nodes,
        #[serde(flatten)]
        pub relays: Relays,
    }

    #[test]
    #[serial]
    fn parse_yaml_config() {
        std::env::set_var("NODE_NAME", "node1");
        let mut contents = r#"
            variables:
              var_s: node2
              var_i: 1 # comment
            nodes:
              # comment
              - $NODE_NAME
              - $var_s
            relays:
              r1:
                at: /project/default
                to: outlet-node
        "#
        .to_string();

        let parsed = ConfigParser::parse::<TestConfig>(&mut contents).unwrap();
        let nodes = parsed.nodes.into_parsed_commands().unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].name, "node1");
        assert_eq!(nodes[1].name, "node2");
    }

    #[test]
    #[serial]
    fn parse_json_config() {
        std::env::set_var("NODE_NAME", "node1");
        let mut contents = r#"
            {
                # comment
                "variables": {
                    "var_s": "node2",
                    "var_i": 1, # trailing commas are supported
                },
                "nodes": [
                    "$NODE_NAME",
                    "$var_s",
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
        let parsed = ConfigParser::parse::<TestConfig>(&mut contents).unwrap();
        let nodes = parsed.nodes.into_parsed_commands().unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].name, "node1");
        assert_eq!(nodes[1].name, "node2");
    }

    #[test]
    fn parse_without_variables_section() {
        let mut contents = r#"
            {
                "nodes": [
                    "node1",
                    "node2",
                ],
            }
        "#
        .to_string();
        let parsed = ConfigParser::parse::<TestConfig>(&mut contents).unwrap();
        let nodes = parsed.nodes.into_parsed_commands().unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].name, "node1");
        assert_eq!(nodes[1].name, "node2");
    }

    #[test]
    fn parse_from_inline_yaml() {
        let mut contents = "relay: r1".to_string();
        let parsed = ConfigParser::parse::<TestConfig>(&mut contents).unwrap();
        let nodes = parsed.relays.into_parsed_commands(None).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].relay_name, "r1");
    }
}
