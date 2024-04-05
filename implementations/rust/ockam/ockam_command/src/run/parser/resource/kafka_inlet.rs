use crate::kafka::inlet::create::CreateCommand;
use crate::run::parser::building_blocks::{ArgsToCommands, UnnamedResources};
use crate::run::parser::resource::traits::CommandsParser;
use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::run::parser::resource::ValuesOverrides;
use crate::{kafka::inlet, Command, OckamSubcommand};
use async_trait::async_trait;
use miette::{miette, Result};
use ockam_api::colors::color_primary;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KafkaInlet {
    #[serde(alias = "kafka-inlet")]
    pub kafka_inlet: Option<UnnamedResources>,
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
}

#[async_trait]
impl CommandsParser<CreateCommand> for KafkaInlet {
    fn parse_commands(self, overrides: &ValuesOverrides) -> Result<Vec<CreateCommand>> {
        match self.kafka_inlet {
            Some(c) => {
                let mut cmds = c.into_commands(Self::get_subcommand)?;
                if let Some(node_name) = overrides.override_node_name.as_ref() {
                    for cmd in cmds.iter_mut() {
                        cmd.node_opts.at_node = Some(node_name.clone())
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
    use ockam_core::env::FromString;
    use ockam_multiaddr::MultiAddr;
    use std::net::SocketAddr;
    use std::str::FromStr;

    #[test]
    fn kafka_inlet_config() {
        let named = r#"
            kafka-inlet:
              from: 127.0.0.1:9092
              to: /project/default
              at: node_name
        "#;
        let parsed: KafkaInlet = serde_yaml::from_str(named).unwrap();
        let cmds = parsed.parse_commands(&ValuesOverrides::default()).unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(
            cmds[0].from,
            SocketAddr::from_str("127.0.0.1:9092").unwrap()
        );
        assert_eq!(
            cmds[0].to.as_ref().unwrap(),
            &MultiAddr::from_string("/project/default").unwrap(),
        );
        assert_eq!(cmds[0].node_opts.at_node.as_ref().unwrap(), "node_name");

        let unnamed = r#"
            kafka-inlet:
              bootstrap-server: my-kafka.example.com:9092
              consumer: /dnsaddr/kafka-outlet.local/tcp/5000
        "#;
        let parsed: KafkaInlet = serde_yaml::from_str(unnamed).unwrap();
        let cmds = parsed.parse_commands(&ValuesOverrides::default()).unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(
            cmds[0].bootstrap_server.as_ref().unwrap(),
            "my-kafka.example.com:9092"
        );
        assert_eq!(
            cmds[0].consumer.as_ref().unwrap(),
            &MultiAddr::from_string("/dnsaddr/kafka-outlet.local/tcp/5000").unwrap(),
        );
        assert!(cmds[0].node_opts.at_node.is_none());
    }
}
