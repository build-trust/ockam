use miette::{miette, Result};
use ockam_api::colors::color_primary;
use serde::{Deserialize, Serialize};

use crate::kafka::outlet;
use crate::kafka::outlet::create::CreateCommand;
use crate::run::parser::building_blocks::{ArgsToCommands, ResourceNameOrMap};

use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::{Command, OckamSubcommand};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct KafkaOutlet {
    #[serde(alias = "kafka-outlets", alias = "kafka-outlet")]
    pub kafka_outlet: Option<ResourceNameOrMap>,
}

impl KafkaOutlet {
    fn get_subcommand(args: &[String]) -> Result<CreateCommand> {
        if let OckamSubcommand::KafkaOutlet(cmd) = parse_cmd_from_args(CreateCommand::NAME, args)? {
            #[allow(irrefutable_let_patterns)]
            if let outlet::KafkaOutletSubcommand::Create(c) = cmd.subcommand {
                return Ok(c);
            }
        }
        Err(miette!(format!(
            "Failed to parse {} command",
            color_primary(CreateCommand::NAME)
        )))
    }

    pub fn into_parsed_commands(
        self,
        default_node_name: Option<&String>,
    ) -> Result<Vec<CreateCommand>> {
        match self.kafka_outlet {
            Some(c) => {
                let mut cmds = c.into_commands(Self::get_subcommand)?;
                if let Some(node_name) = default_node_name {
                    for cmd in cmds.iter_mut() {
                        if cmd.node_opts.at_node.is_none() {
                            cmd.node_opts.at_node = Some(node_name.to_string())
                        }
                    }
                }
                Ok(cmds)
            }
            None => Ok(vec![]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam::transport::SchemeHostnamePort;

    #[test]
    fn kafka_outlet_config() {
        let unnamed = r#"
            kafka-outlet:
              bootstrap-server: 192.168.0.100:9092
              at: node_name
        "#;
        let parsed: KafkaOutlet = serde_yaml::from_str(unnamed).unwrap();
        let default_node_name = "n1".to_string();
        let cmds = parsed
            .into_parsed_commands(Some(&default_node_name))
            .unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(
            cmds[0].bootstrap_server,
            SchemeHostnamePort::new("tcp", "192.168.0.100", 9092).unwrap(),
        );
        assert_eq!(cmds[0].node_opts.at_node.as_ref().unwrap(), "node_name");

        let named = r#"
            kafka-outlet:
              ko:
                bootstrap-server: 192.168.0.100:9092
        "#;
        let parsed: KafkaOutlet = serde_yaml::from_str(named).unwrap();
        let default_node_name = "n1".to_string();
        let cmds = parsed
            .into_parsed_commands(Some(&default_node_name))
            .unwrap();
        assert_eq!(cmds[0].name, "ko".to_string());
        assert_eq!(cmds[0].node_opts.at_node.as_ref(), Some(&default_node_name));

        // check if the default node name is used when the configuration does not specify it
        let list = r#"
            kafka-outlet:
              - bootstrap-server: 192.168.0.100:9092
        "#;
        let parsed: KafkaOutlet = serde_yaml::from_str(list).unwrap();
        let default_node_name = "n1".to_string();
        let cmds = parsed
            .into_parsed_commands(Some(&default_node_name))
            .unwrap();
        assert_eq!(cmds[0].node_opts.at_node.as_ref(), Some(&default_node_name));
    }
}
