use crate::kafka::inlet::create::CreateCommand;
use crate::run::parser::building_blocks::{ArgsToCommands, ResourceNameOrMap};

use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::{kafka::inlet, Command, OckamSubcommand};
use miette::{miette, Result};
use ockam_api::colors::color_primary;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct KafkaInlet {
    #[serde(alias = "kafka-inlets", alias = "kafka-inlet")]
    pub kafka_inlet: Option<ResourceNameOrMap>,
}

impl KafkaInlet {
    fn get_subcommand(args: &[String]) -> Result<CreateCommand> {
        if let OckamSubcommand::KafkaInlet(cmd) = parse_cmd_from_args(CreateCommand::NAME, args)? {
            #[allow(irrefutable_let_patterns)]
            if let inlet::KafkaInletSubcommand::Create(c) = cmd.subcommand {
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
        match self.kafka_inlet {
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

    use ockam_core::env::FromString;
    use ockam_multiaddr::MultiAddr;

    #[test]
    fn kafka_inlet_config() {
        let unnamed = r#"
            kafka-inlet:
              from: 127.0.0.1:9092
              to: /project/default
              consumer-relay: /ip4/192.168.1.1/tcp/4000
              publishing-relay: /ip4/192.168.1.2/tcp/4000
              at: node_name
              encrypted-fields:
                - one
                - two
        "#;
        let parsed: KafkaInlet = serde_yaml::from_str(unnamed).unwrap();
        let default_node_name = "n1".to_string();
        let cmds = parsed
            .into_parsed_commands(Some(&default_node_name))
            .unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(
            cmds[0].from,
            SchemeHostnamePort::new("tcp", "127.0.0.1", 9092).unwrap()
        );
        assert_eq!(
            &cmds[0].to,
            &MultiAddr::from_string("/project/default").unwrap(),
        );
        assert_eq!(
            cmds[0].consumer_relay.as_ref().unwrap(),
            &MultiAddr::from_string("/ip4/192.168.1.1/tcp/4000").unwrap(),
        );
        assert_eq!(
            cmds[0].publishing_relay.as_ref().unwrap(),
            &MultiAddr::from_string("/ip4/192.168.1.2/tcp/4000").unwrap(),
        );
        assert_eq!(cmds[0].node_opts.at_node, Some("node_name".to_string()));
        assert!(!cmds[0].no_publishing);

        assert_eq!(
            cmds[0].encrypted_fields,
            vec!["one".to_string(), "two".to_string()]
        );

        let named = r#"
            kafka-inlet:
              ki:
                consumer: /dnsaddr/kafka-outlet.local/tcp/5000
                avoid-publishing: true
        "#;
        let parsed: KafkaInlet = serde_yaml::from_str(named).unwrap();
        let cmds = parsed
            .into_parsed_commands(Some(&default_node_name))
            .unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name, "ki");
        assert_eq!(
            cmds[0].consumer.as_ref().unwrap(),
            &MultiAddr::from_string("/dnsaddr/kafka-outlet.local/tcp/5000").unwrap(),
        );
        assert!(cmds[0].no_publishing);
        assert_eq!(cmds[0].node_opts.at_node, Some(default_node_name.clone()));

        let list = r#"
            kafka-inlet:
              - consumer: /dnsaddr/kafka-outlet.local/tcp/5000
                avoid-publishing: true
        "#;
        let parsed: KafkaInlet = serde_yaml::from_str(list).unwrap();
        let cmds = parsed
            .into_parsed_commands(Some(&default_node_name))
            .unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(
            cmds[0].consumer.as_ref().unwrap(),
            &MultiAddr::from_string("/dnsaddr/kafka-outlet.local/tcp/5000").unwrap(),
        );
        assert!(cmds[0].no_publishing);
        assert_eq!(cmds[0].node_opts.at_node, Some(default_node_name));
    }
}
