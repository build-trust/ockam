use miette::{miette, Result};
use ockam_api::colors::color_primary;
use serde::{Deserialize, Serialize};

use crate::run::parser::building_blocks::{ArgsToCommands, ResourceNameOrMap};

use crate::influxdb::inlet::create::InfluxDBCreateCommand;
use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::{influxdb::inlet, Command, OckamSubcommand};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InfluxDBInlets {
    #[serde(alias = "influxdb-inlets", alias = "influxdb-inlet")]
    pub influxdb_inlets: Option<ResourceNameOrMap>,
}

impl InfluxDBInlets {
    fn get_subcommand(args: &[String]) -> Result<InfluxDBCreateCommand> {
        if let OckamSubcommand::InfluxDBInlet(cmd) =
            parse_cmd_from_args(InfluxDBCreateCommand::NAME, args)?
        {
            let inlet::InfluxDBInletSubCommand::Create(c) = cmd.subcommand;
            return Ok(c);
        }
        Err(miette!(format!(
            "Failed to parse {} command",
            color_primary(InfluxDBCreateCommand::NAME)
        )))
    }

    pub fn into_parsed_commands(
        self,
        default_node_name: Option<&String>,
    ) -> Result<Vec<InfluxDBCreateCommand>> {
        match self.influxdb_inlets {
            Some(c) => {
                let mut cmds =
                    c.into_commands_with_name_arg(Self::get_subcommand, Some("alias"))?;
                if let Some(node_name) = default_node_name.as_ref() {
                    for cmd in cmds.iter_mut() {
                        if cmd.tcp_inlet.at.is_none() {
                            cmd.tcp_inlet.at = Some(node_name.to_string())
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
    use ockam::transport::HostnamePort;

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
                alias: my_inlet
                lease-manager-route: /service/test
        "#;
        let parsed: InfluxDBInlets = serde_yaml::from_str(named).unwrap();
        let default_node_name = "n1".to_string();
        let cmds = parsed
            .into_parsed_commands(Some(&default_node_name))
            .unwrap();
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].tcp_inlet.alias.as_ref().unwrap(), "ti1");
        assert_eq!(cmds[0].tcp_inlet.from, HostnamePort::new("127.0.0.1", 6060));
        assert_eq!(cmds[0].tcp_inlet.at.as_ref().unwrap(), "n");
        assert_eq!(cmds[1].tcp_inlet.alias.as_ref().unwrap(), "my_inlet");
        assert_eq!(cmds[1].tcp_inlet.from, HostnamePort::new("127.0.0.1", 6061));
        assert_eq!(cmds[1].tcp_inlet.at.as_ref(), Some(&default_node_name));

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
        assert_eq!(cmds[0].tcp_inlet.from, HostnamePort::new("127.0.0.1", 6060));
        assert_eq!(cmds[0].tcp_inlet.at.as_ref().unwrap(), "n");
        assert_eq!(cmds[1].tcp_inlet.from, HostnamePort::new("127.0.0.1", 6061));
        assert_eq!(cmds[1].tcp_inlet.at.as_ref(), Some(&default_node_name));
    }
}
