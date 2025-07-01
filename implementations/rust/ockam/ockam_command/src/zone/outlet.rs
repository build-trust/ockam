use crate::cluster::common_args::{ClusterArg, HttpApiArgs};
use crate::cluster::utils::get_api_client;
use crate::node::config::ConfigArgs;
use crate::node::node_callback::NodeCallback;
use crate::node::util::wait_for_node_callback_future;
use crate::node_command::InMemoryNodeCommand;
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
use ockam_api::address::extract_address_value;
use ockam_api::nodes::InMemoryNode;
use ockam_api::CliState;
use ockam_node::Context;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

const LONG_ABOUT: &str = include_str!("./static/outlet/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/outlet/after_long_help.txt");

/// Open a portal outlet
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct OutletCommand {
    #[command(flatten)]
    pub cluster: ClusterArg,

    #[command(flatten)]
    pub zone: ZoneNameOrConfigArg,

    // == Node Options ==
    #[command(flatten)]
    pub enrollment_ticket: EnrollmentTicketConfigArg,

    /// Relay to register at.
    #[arg(long)]
    pub relay: Option<String>,

    /// Returns the tokio handle of task running the outlet.
    #[arg(long, hide = true)]
    pub background: bool,

    #[command(flatten)]
    pub http_api: HttpApiArgs,

    // == TCP Outlet Options ==
    /// Service address of your TCP Outlet, which is part of a route used in other commands.
    /// This unique address identifies the TCP Outlet worker on the Node on your local machine.
    /// Examples are `/service/my-outlet` or `my-outlet`.
    /// If not provided, the name of the relay will be used.
    #[arg(long, display_order = 902, id = "OUTLET_ADDRESS", value_parser = extract_address_value)]
    pub from: Option<String>,

    /// Network address where your application is listening to.
    /// Your TCP Outlet will forward raw TCP traffic to this destination.
    #[arg(long, id = "SOCKET_ADDRESS", display_order = 900, value_parser = hostname_parser)]
    pub to: Option<SchemeHostnamePort>,

    #[arg(help = docs::about("\
    Policy expression that will be used for access control to the TCP Outlet. \
    If you don't provide it, the policy set for the \"tcp-outlet\" resource type will be used. \
    \n\nYou can check the fallback policy with `ockam policy show --resource-type tcp-outlet`"))]
    #[arg(
        long,
        visible_alias = "expression",
        display_order = 904,
        id = "POLICY_EXPRESSION"
    )]
    pub allow: Option<PolicyExpression>,
}

#[derive(Clone)]
struct OutletNodeCommand {
    opts: CommandGlobalOpts,
    command: OutletCommand,
}

#[async_trait]
impl InMemoryNodeCommand for OutletNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let ctx = node.ctx();
        let use_http_api = self.command.http_api.use_http_api();
        let api_client = get_api_client(&node, use_http_api).await?;

        // TODO: the cluster and zone are only needed here if the enrollment ticket is not provided
        //  *but* at some point we will need them to set the default policy on the outlet
        let cluster = self.command.cluster.get_cluster(ctx, &node).await?;
        let zone_config = self.command.zone.zone_config()?;
        let zone_name = zone_config.name;
        let relay_name = format!("{}-{}-{}", cluster, zone_name, self.command.relay());
        let enrollment_ticket = self
            .command
            .enrollment_ticket
            .get(
                ctx,
                &*api_client,
                &cluster,
                &zone_name,
                Some(self.command.relay().to_string()),
            )
            .await?;
        let mut node_config = serde_json::json!({
            "relay": relay_name,
            "tcp-outlet": {
              "to": self.command.to().to_string(),
            }
        });
        let from = self.command.from.as_deref().unwrap_or(self.command.relay());
        node_config["tcp-outlet"]["from"] = from.to_string().into();

        if let Some(allow) = &self.command.allow {
            node_config["tcp-outlet"]["allow"] = allow.to_string().into();
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
                    let addr = self.command.to().to_string();
                    if let Err(e) = tokio::net::TcpStream::connect(&addr).await {
                        Err(miette::miette!(e).wrap_err(miette::miette!("Outlet failed to start at {}", addr)))
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
impl Command for OutletCommand {
    const NAME: &'static str = "zone outlet";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let command = self.parse_args()?;
        let command = OutletNodeCommand {
            opts: opts.clone(),
            command,
        };
        command.execute(ctx, opts.state.clone()).await?;
        Ok(())
    }
}

impl OutletCommand {
    fn to(&self) -> &SchemeHostnamePort {
        self.to
            .as_ref()
            .expect("Pod name should be set by parse_args")
    }

    fn relay(&self) -> &str {
        self.relay
            .as_ref()
            .expect("From address should be set by parse_args")
    }

    pub fn parse_args(mut self) -> Result<Self> {
        // At least `--to` or `--relay` must be provided
        if self.to.is_none() && self.relay.is_none() {
            return Err(miette::miette!(
                "You must provide at least the `--to` or the `--relay` argument."
            ));
        }

        // Try to derive `--to` or `--relay` from the other one
        let zone_config = self.zone.zone_config().unwrap_or_default();
        'l: for pod in zone_config.pods.iter() {
            for inlet in pod.portals.inlets.iter() {
                if let Some(to) = &self.to {
                    let from = SchemeHostnamePort::from_str(&inlet.from)?;
                    if &from == to {
                        self.relay = inlet.name.clone();
                        break 'l;
                    }
                } else if let Some(relay) = &self.relay {
                    if let Some(inlet_name) = &inlet.name {
                        if inlet_name == relay {
                            self.to = Some(SchemeHostnamePort::from_str(&inlet.from)?);
                            break 'l;
                        }
                    }
                }
            }
        }

        // If `--to` is not set at this point, return an error
        if self.to.is_none() {
            return Err(miette::miette!(
                "Couldn't determine a value for `--to`. Please provide the argument explicitly."
            ));
        }

        // If `--relay` is not set at this point, return an error
        if self.relay.is_none() {
            return Err(miette::miette!(
                "Couldn't determine a value for `--relay`. Please provide the argument explicitly."
            ));
        }

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_args {
        use super::*;

        use crate::zone::common_args::ZoneNameOrConfigArg;
        use crate::zone::zone_config::{Inlet, Pod, Portals, ZoneConfig};
        use ockam::transport::SchemeHostnamePort;
        use std::str::FromStr;

        fn create_test_command() -> OutletCommand {
            OutletCommand {
                cluster: ClusterArg::default(),
                zone: ZoneNameOrConfigArg::default(),
                enrollment_ticket: EnrollmentTicketConfigArg::default(),
                relay: None,
                background: false,
                http_api: HttpApiArgs::default(),
                from: None,
                to: None,
                allow: None,
            }
        }

        fn create_zone_config_with_inlet(
            inlet_name: Option<String>,
            inlet_from: &str,
        ) -> ZoneConfig {
            ZoneConfig {
                name: "zone".to_string(),
                pods: vec![Pod {
                    name: "main-pod".to_string(),
                    portals: Portals {
                        inlets: vec![Inlet {
                            name: inlet_name,
                            from: inlet_from.to_string(),
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                    ..Default::default()
                }],
            }
        }

        #[test]
        fn test_parse_args_with_both_to_and_relay() {
            let mut command = create_test_command();
            command.to = Some(SchemeHostnamePort::from_str("tcp://localhost:8080").unwrap());
            command.relay = Some("my-relay".to_string());

            let result = command.parse_args();
            assert!(result.is_ok());

            let parsed = result.unwrap();
            assert_eq!(parsed.to().to_string(), "tcp://localhost:8080");
            assert_eq!(parsed.relay(), "my-relay");
        }

        #[test]
        fn test_parse_args_with_neither_to_nor_relay() {
            let command = create_test_command();

            let result = command.parse_args();
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("You must provide at least the `--to` or the `--relay` argument"));
        }

        #[test]
        fn test_parse_args_derive_relay_from_to() {
            let mut command = create_test_command();
            command.to = Some(SchemeHostnamePort::from_str("tcp://localhost:8080").unwrap());

            // Mock zone config with matching inlet
            let zone_config = create_zone_config_with_inlet(
                Some("derived-relay".to_string()),
                "tcp://localhost:8080",
            );
            command.zone = ZoneNameOrConfigArg {
                zone_name: None,
                zone_config: Some(serde_yaml::to_string(&zone_config).unwrap()),
            };
            let result = command.parse_args();
            assert!(result.is_ok());

            let parsed = result.unwrap();
            assert_eq!(parsed.to().to_string(), "tcp://localhost:8080");
            assert_eq!(parsed.relay(), "derived-relay");
        }

        #[test]
        fn test_parse_args_derive_to_from_relay() {
            let mut command = create_test_command();
            command.relay = Some("my-relay".to_string());

            // Mock zone config with matching inlet
            let zone_config =
                create_zone_config_with_inlet(Some("my-relay".to_string()), "tcp://localhost:9000");
            command.zone = ZoneNameOrConfigArg {
                zone_name: None,
                zone_config: Some(serde_yaml::to_string(&zone_config).unwrap()),
            };

            let result = command.parse_args();
            assert!(result.is_ok());

            let parsed = result.unwrap();
            assert_eq!(parsed.to().to_string(), "tcp://localhost:9000");
            assert_eq!(parsed.relay(), "my-relay");
        }

        #[test]
        fn test_parse_args_only_to_no_matching_inlet() {
            let mut command = create_test_command();
            command.to = Some(SchemeHostnamePort::from_str("tcp://localhost:8080").unwrap());

            // Zone config with non-matching inlet
            let zone_config = create_zone_config_with_inlet(
                Some("other-relay".to_string()),
                "tcp://localhost:9000",
            );
            command.zone = ZoneNameOrConfigArg {
                zone_name: None,
                zone_config: Some(serde_yaml::to_string(&zone_config).unwrap()),
            };

            let result = command.parse_args();
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("Couldn't determine a value for `--relay`"));
        }

        #[test]
        fn test_parse_args_only_relay_no_matching_inlet() {
            let mut command = create_test_command();
            command.relay = Some("my-relay".to_string());

            // Zone config with non-matching inlet
            let zone_config = create_zone_config_with_inlet(
                Some("other-relay".to_string()),
                "tcp://localhost:9000",
            );
            command.zone = ZoneNameOrConfigArg {
                zone_name: None,
                zone_config: Some(serde_yaml::to_string(&zone_config).unwrap()),
            };

            let result = command.parse_args();
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("Couldn't determine a value for `--to`"));
        }

        #[test]
        fn test_parse_args_inlet_without_name() {
            let mut command = create_test_command();
            command.relay = Some("my-relay".to_string());

            // Zone config with inlet that has no name
            let zone_config = create_zone_config_with_inlet(None, "tcp://localhost:9000");
            command.zone = ZoneNameOrConfigArg {
                zone_name: None,
                zone_config: Some(serde_yaml::to_string(&zone_config).unwrap()),
            };

            let result = command.parse_args();
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("Couldn't determine a value for `--to`"));
        }

        #[test]
        fn test_parse_args_multiple_pods_with_inlets() {
            let mut command = create_test_command();
            command.relay = Some("target-relay".to_string());

            let zone_config = ZoneConfig {
                name: "zone".to_string(),
                pods: vec![
                    Pod {
                        name: "main-pod".to_string(),
                        portals: Portals {
                            inlets: vec![Inlet {
                                name: Some("other-relay".to_string()),
                                from: "tcp://localhost:7000".to_string(),
                                ..Default::default()
                            }],
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    Pod {
                        name: "pod2".to_string(),
                        portals: Portals {
                            inlets: vec![Inlet {
                                name: Some("target-relay".to_string()),
                                from: "tcp://localhost:8000".to_string(),
                                ..Default::default()
                            }],
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ],
            };
            command.zone = ZoneNameOrConfigArg {
                zone_name: None,
                zone_config: Some(serde_yaml::to_string(&zone_config).unwrap()),
            };

            let result = command.parse_args();
            assert!(result.is_ok());

            let parsed = result.unwrap();
            assert_eq!(parsed.to().to_string(), "tcp://localhost:8000");
            assert_eq!(parsed.relay(), "target-relay");
        }

        #[test]
        fn test_to_method_panics_when_none() {
            let command = create_test_command();
            std::panic::catch_unwind(|| {
                command.to();
            })
            .expect_err("Expected panic when to is None");
        }

        #[test]
        fn test_relay_method_panics_when_none() {
            let command = create_test_command();
            std::panic::catch_unwind(|| {
                command.relay();
            })
            .expect_err("Expected panic when relay is None");
        }
    }
}
