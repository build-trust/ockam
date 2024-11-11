use miette::{miette, Result};
use ockam_api::colors::color_primary;
use serde::{Deserialize, Serialize};

use crate::run::parser::building_blocks::{ArgsToCommands, ResourceNameOrMap};

use crate::influxdb::inlet::create::CreateCommand;
use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::{influxdb::inlet, Command, OckamSubcommand};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InfluxDBInlets {
    #[serde(alias = "influxdb-inlets", alias = "influxdb-inlet")]
    pub influxdb_inlets: Option<ResourceNameOrMap>,
}

impl InfluxDBInlets {
    fn get_subcommand(args: &[String]) -> Result<CreateCommand> {
        if let OckamSubcommand::InfluxDBInlet(cmd) = parse_cmd_from_args(CreateCommand::NAME, args)?
        {
            let inlet::InfluxDBInletSubCommand::Create(c) = cmd.subcommand;
            return Ok(c);
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
        match self.influxdb_inlets {
            Some(c) => {
                let mut cmds = c.into_commands(Self::get_subcommand)?;
                if let Some(node_name) = default_node_name.as_ref() {
                    for cmd in cmds.iter_mut() {
                        if cmd.at.is_none() {
                            cmd.at = Some(node_name.to_string())
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
    fn tcp_inlet_config() {
        let named = r#"
            influxdb_inlets:
              ti1:
                from: 6060
                at: n
                lease-manager-route: /service/test
              ti2:
                from: '6061'
                lease-manager-route: /service/test
              ti3:
                from: tls://localhost:6062
                lease-manager-route: /service/test
        "#;
        let parsed: InfluxDBInlets = serde_yaml::from_str(named).unwrap();
        let default_node_name = "n1".to_string();
        let cmds = parsed
            .into_parsed_commands(Some(&default_node_name))
            .unwrap();
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0].name.as_ref().unwrap(), "ti1");
        assert_eq!(
            cmds[0].from,
            SchemeHostnamePort::new("tcp", "127.0.0.1", 6060).unwrap()
        );
        assert_eq!(cmds[0].at.as_ref().unwrap(), "n");
        assert_eq!(cmds[1].name.as_ref().unwrap(), "ti2");
        assert_eq!(
            cmds[1].from,
            SchemeHostnamePort::new("tcp", "127.0.0.1", 6061).unwrap()
        );
        assert_eq!(cmds[1].at.as_ref(), Some(&default_node_name));
        assert_eq!(cmds[2].name.as_ref().unwrap(), "ti3");
        assert_eq!(
            cmds[2].from,
            SchemeHostnamePort::new("tls", "localhost", 6062).unwrap()
        );
        assert_eq!(cmds[2].at.as_ref(), Some(&default_node_name));

        let unnamed = r#"
            influxdb_inlets:
              - from: 6060
                at: n
              - from: '6061'
                lease-manager-route: /service/test
        "#;
        let parsed: InfluxDBInlets = serde_yaml::from_str(unnamed).unwrap();
        let cmds = parsed
            .into_parsed_commands(Some(&default_node_name))
            .unwrap();
        assert_eq!(cmds.len(), 2);
        assert_eq!(
            cmds[0].from,
            SchemeHostnamePort::new("tcp", "127.0.0.1", 6060).unwrap()
        );
        assert_eq!(cmds[0].at.as_ref().unwrap(), "n");
        assert_eq!(
            cmds[1].from,
            SchemeHostnamePort::new("tcp", "127.0.0.1", 6061).unwrap()
        );
        assert_eq!(cmds[1].at.as_ref(), Some(&default_node_name));
    }
}
