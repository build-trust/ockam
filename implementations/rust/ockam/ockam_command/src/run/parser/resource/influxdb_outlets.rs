use miette::{miette, Result};
use ockam_api::colors::color_primary;
use serde::{Deserialize, Serialize};

use crate::run::parser::building_blocks::{ArgsToCommands, ResourceNameOrMap};

use crate::influxdb::outlet::create::InfluxDBCreateCommand;
use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::{influxdb::outlet, Command, OckamSubcommand};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InfluxDBOutlets {
    #[serde(alias = "influxdb-outlets", alias = "influxdb-outlet")]
    pub influxdb_outlets: Option<ResourceNameOrMap>,
}

impl InfluxDBOutlets {
    fn get_subcommand(args: &[String]) -> Result<InfluxDBCreateCommand> {
        if let OckamSubcommand::InfluxDBOutlet(cmd) =
            parse_cmd_from_args(InfluxDBCreateCommand::NAME, args)?
        {
            let outlet::InfluxDBOutletSubCommand::Create(c) = cmd.subcommand;
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
        match self.influxdb_outlets {
            Some(c) => {
                let mut cmds = c.into_commands_with_name_arg(Self::get_subcommand, Some("from"))?;
                if let Some(node_name) = default_node_name {
                    for cmd in cmds.iter_mut() {
                        if cmd.tcp_outlet.at.is_none() {
                            cmd.tcp_outlet.at = Some(node_name.to_string())
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
    fn tcp_outlet_config() {
        let named = r#"
            influxdb_outlets:
              ti1:
                to: 127.0.0.1:6060
                from: my_outlet
                leased-token-permissions: ""
                leased-token-strategy: shared
                leased-token-expires-in: 1h
        "#;
        let parsed: InfluxDBOutlets = serde_yaml::from_str(named).unwrap();
        let default_node_name = "n1".to_string();
        let cmds = parsed
            .into_parsed_commands(Some(&default_node_name))
            .unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].tcp_outlet.to, HostnamePort::new("127.0.0.1", 6060));
    }
}
