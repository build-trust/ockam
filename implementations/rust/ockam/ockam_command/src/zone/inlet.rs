use crate::cluster::common_args::{ClusterArg, HttpApiArgs};
use crate::cluster::utils::get_api_client;
use crate::node::config::ConfigArgs;
use crate::node::node_callback::NodeCallback;
use crate::node::util::wait_for_node_callback_future;
use crate::node_command::InMemoryNodeCommand;
use crate::tcp::inlet::create::tcp_inlet_default_from_addr;
use crate::util::foreground_args::ForegroundArgs;
use crate::util::parsers::hostname_parser;
use crate::zone::common_args::{EnrollmentTicketConfigArg, ZoneNameOrConfigArg};
use crate::zone::watcher::DirectoryWatcher;
use crate::{docs, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use miette::IntoDiagnostic;
use ockam::transport::SchemeHostnamePort;
use ockam_abac::PolicyExpression;
use ockam_api::nodes::InMemoryNode;
use ockam_api::CliState;
use ockam_node::Context;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

const LONG_ABOUT: &str = include_str!("./static/inlet/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/inlet/after_long_help.txt");

/// Open a portal inlet
#[derive(Clone, Debug, Args, Default)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct InletCommand {
    #[command(flatten)]
    pub cluster: ClusterArg,

    #[command(flatten)]
    pub zone: ZoneNameOrConfigArg,

    /// References the name of TCP Outlet created in the Zone and the Relay name.
    #[arg(long)]
    pub pod: Option<String>,

    // == Node Options ==
    #[command(flatten)]
    pub enrollment_ticket: EnrollmentTicketConfigArg,

    #[arg(long)]
    pub background: bool,

    /// Disable the Ctrl-C handler.
    #[arg(long)]
    pub no_ctrlc_handler: bool,

    #[command(flatten)]
    pub http_api: HttpApiArgs,

    // == TCP Inlet Options ==
    /// Address on which to accept TCP connections, in the format `<scheme>://<host>:<port>`.
    /// At least the port must be provided. The default scheme is `tcp` and the default host is `127.0.0.1`.
    #[arg(long, display_order = 900, id = "SOCKET_ADDRESS", hide_default_value = true, value_parser = hostname_parser)]
    pub from: Option<SchemeHostnamePort>,

    /// Name of the TCP Outlet service to connect to.
    #[arg(long, id = "ROUTE")]
    pub to: Option<String>,

    #[arg(help = docs::about("\
     Policy expression that will be used for access control to the TCP Inlet. \
     If you don't provide it, the policy set for the \"tcp-inlet\" resource type will be used. \
     \n\nYou can check the fallback policy with `ockam policy show --resource-type tcp-inlet`."))]
    #[arg(
        long,
        visible_alias = "expression",
        display_order = 900,
        id = "POLICY_EXPRESSION"
    )]
    pub allow: Option<PolicyExpression>,
}

#[derive(Clone)]
struct InletNodeCommand {
    opts: CommandGlobalOpts,
    command: InletCommand,
}

#[async_trait]
impl InMemoryNodeCommand for InletNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let ctx = node.ctx();
        let use_http_api = self.command.http_api.use_http_api();
        let api_client = get_api_client(&node, use_http_api).await?;
        let cluster = self.command.cluster.get_cluster(ctx, &node).await?;
        let zone_config = self.command.zone.zone_config()?;
        let zone_name = zone_config.name;
        let enrollment_ticket = self
            .command
            .enrollment_ticket
            .get(ctx, &*api_client, &cluster, &zone_name, None)
            .await?;
        let relay_name = format!("{}-{}-{}", cluster, zone_name, self.command.pod());
        let outlet_name = self.command.to.as_deref().unwrap_or(self.command.pod());
        let inlet_from = self.command.from();
        let mut node_config = serde_json::json!({
            "tcp-inlet": {
                "from": inlet_from.to_string(),
                "to": outlet_name,
                "via": relay_name
            }
        });
        if let Some(allow) = &self.command.allow {
            node_config["tcp-inlet"]["allow"] = allow.to_string().into();
        }
        let in_memory = true;
        let node_callback = if self.command.background {
            Some(NodeCallback::create().await?)
        } else {
            None
        };
        let node_cmd = crate::node::create::CreateCommand {
            name: node_config.to_string(),
            config_args: ConfigArgs {
                enrollment_ticket: Some(enrollment_ticket),
                ..Default::default()
            },
            foreground_args: ForegroundArgs {
                foreground: true,
                no_ctrlc_handler: self.command.no_ctrlc_handler,
                ..Default::default()
            },
            in_memory,
            tcp_callback_port: node_callback.as_ref().map(|n| n.callback_port()),
            ..Default::default()
        };
        let mut opts = self.opts.clone();
        let handle = tokio::spawn(async move {
            opts.state = Arc::new(CliState::new(in_memory).await?);
            let res = tokio::select! {
                _ = DirectoryWatcher::wait_for_message() => Ok(()),
                res = node_cmd.run(node.ctx(), opts) => res,
            };
            res
        });
        if let Some(node_callback) = node_callback {
            tokio::select! {
                res = wait_for_node_callback_future(handle, node_callback) => {
                    res
                },
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    // Check if the outlet address is reachable or return an error
                    let addr = inlet_from.hostname_port().to_string();
                    if let Err(e) = tokio::net::TcpStream::connect(&addr).await {
                        Err(miette::miette!(e).wrap_err(miette::miette!("Inlet failed to start at {}", addr)))
                    } else {
                        Ok(())
                    }
                }
            }?
        } else {
            handle.await.into_diagnostic()??;
        }
        Ok(())
    }
}

#[async_trait]
impl Command for InletCommand {
    const NAME: &'static str = "zone inlet";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let command = self.parse_args()?;
        let command = InletNodeCommand {
            opts: opts.clone(),
            command,
        };
        command.execute(ctx, opts.state.clone()).await?;
        Ok(())
    }
}

impl InletCommand {
    fn pod(&self) -> &str {
        self.pod
            .as_ref()
            .expect("Pod name should be set by parse_args")
    }

    fn from(&self) -> &SchemeHostnamePort {
        self.from
            .as_ref()
            .expect("From address should be set by parse_args")
    }

    fn parse_args(mut self) -> miette::Result<Self> {
        // At least `--to` or `--pod` must be provided
        if self.to.is_none() && self.pod.is_none() {
            return Err(miette::miette!(
                "You must provide at least the `--to` or the `--pod` argument."
            ));
        }

        // If `--to` is provided, try to derive `--pod` and `--from` if needed
        if let Some(to) = &self.to {
            if self.pod.is_none() || self.from.is_none() {
                let zone_config = self.zone.zone_config().unwrap_or_default();
                'l: for pod in zone_config.pods.iter() {
                    for outlet in pod.portals.outlets.iter() {
                        if let Some(name) = &outlet.name {
                            if to == name {
                                if self.pod.is_none() {
                                    self.pod = outlet.pod_name.clone();
                                }
                                if self.from.is_none() {
                                    self.from = Some(SchemeHostnamePort::from_str(&outlet.to)?);
                                }
                                break 'l;
                            }
                        }
                    }
                }
            }
        }

        // If `--pod` is not set at this point, return an error
        if self.pod.is_none() {
            return Err(miette::miette!(
                "Couldn't determine a value for `--pod`. Please provide the argument explicitly."
            ));
        }

        // If `--from` is not set at this point, set it to the default value
        if self.from.is_none() {
            self.from = Some(tcp_inlet_default_from_addr());
        }

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zone::zone_config::{Outlet, Pod, Portals, ZoneConfig};

    fn create_zone_config(
        pod_name: &str,
        outlet_pod: Option<&str>,
        outlet_name: Option<&str>,
        outlet_to: &str,
    ) -> ZoneConfig {
        let pod = Pod {
            name: pod_name.to_string(),
            portals: Portals {
                outlets: vec![Outlet {
                    pod_name: outlet_pod.map(|p| p.to_string()),
                    name: outlet_name.map(|n| n.to_string()),
                    to: outlet_to.to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            },
            ..Default::default()
        };

        ZoneConfig {
            name: "zone".to_string(),
            pods: vec![pod],
        }
    }

    mod parse_args {
        use super::*;

        #[test]
        fn test_parse_args_missing_required_args() {
            let command = InletCommand::default();
            let result = command.parse_args();
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("You must provide at least the `--to` or the `--pod` argument"));
        }

        #[test]
        fn test_parse_args_with_to_finds_pod_and_config() {
            let zone_config = ZoneConfig::default();
            let zone_arg = ZoneNameOrConfigArg {
                zone_config: Some(serde_yaml::to_string(&zone_config).unwrap()),
                ..Default::default()
            };
            let logs_outlet = zone_config.pods[0].get_outlets().logs;
            assert!(logs_outlet.name.is_some());

            let command = InletCommand {
                to: logs_outlet.name.clone(),
                zone: zone_arg,
                ..Default::default()
            };

            let result = command.parse_args();
            assert!(result.is_ok());
            let parsed = result.unwrap();
            assert_eq!(parsed.pod.unwrap(), logs_outlet.pod_name.unwrap());
            assert_eq!(parsed.to.unwrap(), logs_outlet.name.unwrap());
            assert_eq!(
                parsed.from.unwrap().to_string(),
                format!("tcp://{}", logs_outlet.to)
            );
        }

        #[test]
        fn test_parse_args_with_to_finds_pod_and_no_config() {
            let zone_config = ZoneConfig::default();
            let logs_outlet = zone_config.pods[0].get_outlets().logs;
            assert!(logs_outlet.name.is_some());

            let command = InletCommand {
                to: logs_outlet.name.clone(),
                zone: ZoneNameOrConfigArg::default(),
                ..Default::default()
            };

            let result = command.parse_args();
            assert!(result.is_ok());
            let parsed = result.unwrap();
            assert_eq!(parsed.pod.unwrap(), logs_outlet.pod_name.unwrap());
            assert_eq!(parsed.to.unwrap(), logs_outlet.name.unwrap());
            assert_eq!(
                parsed.from.unwrap().to_string(),
                format!("tcp://{}", logs_outlet.to)
            );
        }

        #[test]
        fn test_parse_args_with_pod_sets_default_from() {
            // Setup command with pod but no from
            let command = InletCommand {
                pod: Some("test_pod".to_string()),
                ..Default::default()
            };

            let result = command.parse_args();
            assert!(result.is_ok());
            let parsed = result.unwrap();
            assert!(parsed.from.is_some());
            assert_eq!(parsed.from.unwrap(), tcp_inlet_default_from_addr());
        }

        #[test]
        fn test_parse_args_with_to_but_no_matching_pod() {
            // Setup command with to, but zone config has no matching outlet
            let command = InletCommand {
                to: Some("tcp://example.com:5000".to_string()),
                zone: ZoneNameOrConfigArg {
                    zone_config: Some(
                        serde_yaml::to_string(&create_zone_config(
                            "main-pod",
                            None,
                            None,
                            "tcp://other.com:6000",
                        ))
                        .unwrap(),
                    ),
                    ..Default::default()
                },
                ..Default::default()
            };

            let result = command.parse_args();
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("Couldn't determine a value for `--pod`"));
        }

        #[test]
        fn test_parse_args_keeps_explicit_values() {
            // Setup command with both to and pod explicitly provided
            let command = InletCommand {
                to: Some("tcp://example.com:4000".to_string()),
                pod: Some("explicit_pod".to_string()),
                from: Some(SchemeHostnamePort::from_str("tcp://localhost:1234").unwrap()),
                ..Default::default()
            };

            let result = command.parse_args();
            assert!(result.is_ok());
            let parsed = result.unwrap();
            assert_eq!(parsed.pod.unwrap(), "explicit_pod");
            assert_eq!(parsed.from.unwrap().to_string(), "tcp://localhost:1234");
        }
    }
}
