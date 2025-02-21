use std::collections::BTreeMap;

use miette::{miette, IntoDiagnostic, Result};
use ockam_api::colors::color_primary;
use serde::{Deserialize, Serialize};

use crate::node::CreateCommand;
use crate::run::parser::building_blocks::{as_command_args, ArgKey, ArgValue, NamedResources};

use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::run::parser::resource::Resource;
use crate::service::config::ServicesConfig;
use crate::{node, Command, OckamSubcommand};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Node {
    pub name: Option<ArgValue>,
    #[serde(alias = "skip-is-running-check")]
    pub skip_is_running_check: Option<ArgValue>,
    pub foreground: Option<ArgValue>,
    #[serde(alias = "child-process")]
    pub child_process: Option<ArgValue>,
    #[serde(alias = "exit-on-eof")]
    pub exit_on_eof: Option<ArgValue>,
    #[serde(alias = "tcp-listener-address")]
    pub tcp_listener_address: Option<ArgValue>,
    #[serde(alias = "http-server", alias = "enable-http-server")]
    pub http_server: Option<ArgValue>,
    #[serde(alias = "no-status-endpoint")]
    pub no_status_endpoint: Option<ArgValue>,
    #[serde(alias = "status-endpoint-port")]
    pub status_endpoint_port: Option<ArgValue>,
    pub identity: Option<ArgValue>,
    pub project: Option<ArgValue>,
    #[serde(flatten, alias = "launch-config")]
    pub services: Option<Services>,
    #[serde(alias = "opentelemetry-context")]
    pub opentelemetry_context: Option<ArgValue>,
    pub udp: Option<ArgValue>,
    #[serde(alias = "udp-listener-address")]
    pub udp_listener_address: Option<ArgValue>,
    #[serde(alias = "in-memory")]
    pub in_memory: Option<ArgValue>,
}

impl Resource<CreateCommand> for Node {
    const COMMAND_NAME: &'static str = CreateCommand::NAME;

    fn args(self) -> Vec<String> {
        let mut args: BTreeMap<ArgKey, ArgValue> = BTreeMap::new();
        args.insert("started-from-configuration".into(), true.into());
        if let Some(name) = self.name {
            args.insert("name".into(), name);
        }
        if let Some(skip_is_running_check) = self.skip_is_running_check {
            args.insert("skip-is-running-check".into(), skip_is_running_check);
        }
        if let Some(foreground) = self.foreground {
            args.insert("foreground".into(), foreground);
        }
        if let Some(child_process) = self.child_process {
            args.insert("child-process".into(), child_process);
        }
        if let Some(exit_on_eof) = self.exit_on_eof {
            args.insert("exit-on-eof".into(), exit_on_eof);
        }
        if let Some(tcp_listener_address) = self.tcp_listener_address {
            args.insert("tcp-listener-address".into(), tcp_listener_address);
        }
        if let Some(udp_listener_address) = self.udp_listener_address {
            args.insert("udp-listener-address".into(), udp_listener_address);
        }
        if let Some(no_status_endpoint) = self.no_status_endpoint {
            args.insert("no-status-endpoint".into(), no_status_endpoint);
        }
        if let Some(status_endpoint_port) = self.status_endpoint_port {
            args.insert("status-endpoint-port".into(), status_endpoint_port);
        }
        if let Some(identity) = self.identity {
            args.insert("identity".into(), identity);
        }
        if let Some(project) = self.project {
            args.insert("project".into(), project);
        }
        if let Some(services) = self.services {
            if let Ok(services) = services.into_arg() {
                if let Ok(services) = serde_json::to_string(&services) {
                    args.insert("services".into(), services.into());
                }
            }
        }
        if let Some(opentelemetry_context) = self.opentelemetry_context {
            args.insert("opentelemetry-context".into(), opentelemetry_context);
        }
        if let Some(udp) = self.udp {
            args.insert("udp".into(), udp);
        }
        if let Some(in_memory) = self.in_memory {
            args.insert("in-memory".into(), in_memory);
        }

        if args.is_empty() {
            return vec![];
        }

        // Convert the map into a list of cli args
        let mut cmd_args = vec![];
        // Remove "name" from the arguments and use it as a positional argument
        if let Some(ArgValue::String(name)) = args.remove(&Self::NAME_ARG.into()) {
            cmd_args.push(name.to_string());
        }
        cmd_args.extend(as_command_args(args));
        cmd_args
    }
}

impl Node {
    pub const NAME_ARG: &'static str = "name";

    /// Return the node name if defined
    pub fn name(&self) -> Option<String> {
        self.name
            .clone()
            .map(|v| match v {
                ArgValue::String(s) => Some(s),
                _ => None,
            })
            .unwrap_or(None)
    }

    /// Return the identity name if defined
    pub fn identity(&self) -> Option<String> {
        self.identity
            .clone()
            .map(|v| match v {
                ArgValue::String(s) => Some(s),
                _ => None,
            })
            .unwrap_or(None)
    }

    pub fn into_parsed_commands(self) -> Result<Vec<CreateCommand>> {
        Ok(vec![Self::get_subcommand(&self.args())?])
    }

    fn get_subcommand(args: &[String]) -> Result<CreateCommand> {
        if let OckamSubcommand::Node(cmd) = parse_cmd_from_args(CreateCommand::NAME, args)? {
            if let node::NodeSubcommand::Create(mut c) = cmd.subcommand {
                c.config_args.configuration = None;
                c.config_args.variables = vec![];
                c.config_args.enrollment_ticket = None;
                c.config_args.started_from_configuration = true;
                return Ok(c);
            }
        }
        Err(miette!(format!(
            "Failed to parse {} command",
            color_primary(CreateCommand::NAME)
        )))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Services {
    #[serde(alias = "start-default-services")]
    pub start_default_services: Option<ArgValue>,
    pub services: Option<NamedResources>,
}

impl Services {
    /// Parse the services fields into a `ServicesConfig` struct
    pub fn into_arg(self) -> Result<ServicesConfig> {
        let mut as_json = serde_json::json!({
            "services": self.services,
        });
        if let Some(start_default_services) = self.start_default_services {
            as_json["start-default-services"] = serde_json::json!(start_default_services);
        }
        serde_json::from_value(as_json).into_diagnostic()
    }

    pub fn from_arg(arg: &ServicesConfig) -> Result<Self> {
        Self::from_string(&arg.to_string()?)
    }

    fn from_string(contents: &str) -> Result<Self> {
        if let Ok(c) = serde_yaml::from_str(contents) {
            return Ok(c);
        }
        if let Ok(c) = serde_json::from_str(contents) {
            return Ok(c);
        }
        Err(miette!(format!("invalid services config {:?}", contents)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::config::ControlApiNodeResolution;

    #[test]
    fn node_config() {
        let test = |c: &str| {
            let parsed: Node = serde_yaml::from_str(c).unwrap();
            let cmds = parsed.into_parsed_commands().unwrap();
            assert_eq!(cmds.len(), 1);
            let cmd = cmds.into_iter().next().unwrap();
            assert_eq!(cmd.name, "n1");
        };

        // Name only
        let config = r#"
            name: n1
        "#;
        test(config);

        // Multiple arguments
        let config = r#"
        name: n1
        tcp-listener-address: 127.0.0.1:1234
        skip-is-running-check: true
        "#;
        test(config);

        // Services
        let config = r#"
        name: n1
        tcp-listener-address: 127.0.0.1:3333
        start_default_services: true
        services:
          startup_services:
            control_api:
              authentication_token: token
              backend: true
              node_resolution: direct-connection
        "#;
        test(config);

        // With other sections
        let config = r#"
        relays: r1

        name: n1
        tcp-listener-address: 127.0.0.1:1234
        skip-is-running-check: true

        tcp_inlets:
          ti1:
            from: 6060
            at: n
        "#;
        test(config);
    }

    #[test]
    fn services_config() {
        let test = |c: &str| {
            // Convert yaml yo struct representation
            let parsed: Services = serde_yaml::from_str(c).unwrap();
            // Convert yaml struct to command argument representation
            let arg = parsed.clone().into_arg().unwrap();
            // Convert command argument representation back to yaml struct representation
            let parsed_again = Services::from_arg(&arg).unwrap();
            // Convert yaml struct representation back to command argument representation to compare
            let arg_again = parsed_again.clone().into_arg().unwrap();
            // We compare the `ServicesConfig` struct as it's easier to compare basic types (bools, strings, etc)
            // than comparing the `Services` struct which has a `NamedResources` field and the types can be dynamic
            assert_eq!(arg, arg_again);
        };

        // Start default services
        let config = r#"
        start-default-services: true
        "#;
        test(config);

        // Single service
        let config = r#"
        services:
          secure-channel-listener:
            address: api
        "#;
        test(config);

        let config = r#"
        services:
          control-api:
            authentication-token: token
            backend: true
            node-resolution: direct-connection
        "#;
        test(config);

        // Multiple services
        let config = r#"
        start-default-services: true
        services:
          secure-channel-listener:
            address: api
            disabled: true
          control-api:
            authentication-token: token
            backend: true
            node-resolution: direct-connection
        "#;
        test(config);

        // Check values
        let parsed: Services = serde_yaml::from_str(config).unwrap();
        let arg = parsed.clone().into_arg().unwrap();
        assert!(arg.start_default_services);

        let services = arg.services.unwrap();
        let secure_channel_listener = services.secure_channel_listener.unwrap();
        assert_eq!(secure_channel_listener.address, "api");
        assert!(secure_channel_listener.disabled);

        let control_api = services.control_api.unwrap();
        assert_eq!(control_api.authentication_token.unwrap(), "token");
        assert!(control_api.backend);
        assert_eq!(
            control_api.node_resolution,
            ControlApiNodeResolution::DirectConnection
        );
    }
}
