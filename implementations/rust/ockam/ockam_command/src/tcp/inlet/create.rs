use crate::node::util::initialize_default_node;
use crate::shared_args::OptionalTimeoutArg;
use crate::tcp::util::alias_parser;
use crate::util::parsers::duration_parser;
use crate::util::parsers::hostname_parser;
use crate::util::parsers::http_header_parser;
use crate::util::{port_is_free_guard, print_warning_for_deprecated_flag_replaced};
use crate::{docs, Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::builder::FalseyValueParser;
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use ockam::identity::Identifier;
use ockam::transport::SchemeHostnamePort;
use ockam::Context;
use ockam_abac::PolicyExpression;
use ockam_api::address::extract_address_value;
use ockam_api::cli_state::journeys::{
    JourneyEvent, NODE_NAME, TCP_INLET_ALIAS, TCP_INLET_AT, TCP_INLET_CONNECTION_STATUS,
    TCP_INLET_FROM, TCP_INLET_TO,
};
use ockam_api::cli_state::{random_name, CliState};
use ockam_api::colors::{color_primary, color_primary_alt};
use ockam_api::nodes::models::portal::InletStatus;
use ockam_api::nodes::service::tcp_inlets::Inlets;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{fmt_info, fmt_log, fmt_ok, fmt_warn, ConnectionStatus};
use ockam_core::api::{Reply, Status};
use ockam_core::{route, Address};
use ockam_multiaddr::proto;
use ockam_multiaddr::{MultiAddr, Protocol as _};
use ockam_node::compat::asynchronous::resolve_peer;

use ockam_api::common_api::tcp_inlet_create::{parse_to_address, tcp_inlet_default_to_address};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tracing::trace;

const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create TCP Inlets
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct CreateCommand {
    /// Assign a name to this TCP Inlet
    #[arg(id = "NAME", value_parser = alias_parser)]
    pub name: Option<String>,

    /// Node on which to start the TCP Inlet.
    #[arg(long, display_order = 900, id = "NODE_NAME", value_parser = extract_address_value)]
    pub at: Option<String>,

    /// Address on which to accept TCP connections, in the format `<scheme>://<host>:<port>`.
    /// At least the port must be provided. The default scheme is `tcp` and the default host is `127.0.0.1`.
    /// If the argument is not set, a random port will be used on the default address `tcp://127.0.0.1`.
    ///
    /// To enable TLS, the `ockam-tls-certificate` credential attribute is required.
    /// It will use the default project TLS certificate provider `/project/default/service/tls_certificate_provider`.
    /// To specify a different certificate provider, use `--tls-certificate-provider`.
    #[arg(long, display_order = 900, id = "SOCKET_ADDRESS", hide_default_value = true, default_value_t = tcp_inlet_default_from_addr(), value_parser = hostname_parser)]
    pub from: SchemeHostnamePort,

    /// Route to a TCP Outlet or the name of the TCP Outlet service you want to connect to.
    ///
    /// If you are connecting to a local node, you can provide the route as `/node/n/service/outlet`.
    ///
    /// If you are connecting to a remote node through a relay in the Orchestrator you can either
    /// provide the full route to the TCP Outlet as `/project/myproject/service/forward_to_myrelay/secure/api/service/outlet`,
    /// or just the service name as `outlet` or `/service/outlet`.
    /// If you are passing just the service name, consider using `--via` to specify the
    /// relay name (e.g. `ockam tcp-inlet create --to outlet --via myrelay`).
    #[arg(long, display_order = 900, id = "ROUTE", default_value_t = tcp_inlet_default_to_address())]
    pub to: String,

    /// Name of the relay that this TCP Inlet will use to connect to the TCP Outlet.
    ///
    /// Use this flag when you are using `--to` to specify the service name of a TCP Outlet
    /// that is reachable through a relay in the Orchestrator.
    /// If you don't provide it, the default relay name will be used, if necessary.
    #[arg(long, display_order = 900, id = "RELAY_NAME")]
    pub via: Option<String>,

    /// Identity to be used to create the secure channel. If not set, the node's identity will be used.
    #[arg(long, value_name = "IDENTITY_NAME", display_order = 900)]
    pub identity: Option<String>,

    /// Restrict access to the TCP Inlet to the provided identity.
    /// When omitted, all identities are allowed.
    #[arg(long, name = "AUTHORIZED", display_order = 900)]
    pub authorized: Option<Identifier>,

    /// [DEPRECATED] Use the <NAME> positional argument instead
    #[arg(long, display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    pub alias: Option<String>,

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

    /// Time to wait for the outlet to be available.
    #[arg(long, display_order = 900, id = "WAIT", default_value = "5s", value_parser = duration_parser)]
    pub connection_wait: Duration,

    /// Time to wait before retrying to connect to the TCP Outlet.
    #[arg(long, display_order = 900, id = "RETRY", default_value = "20s", value_parser = duration_parser)]
    pub retry_wait: Duration,

    #[command(flatten)]
    pub timeout: OptionalTimeoutArg,

    /// Create the TCP Inlet without waiting for the TCP Outlet to connect
    #[arg(long, default_value = "false")]
    pub no_connection_wait: bool,

    /// [DEPRECATED] Use the `udp` scheme in the `--from` argument.
    #[arg(
        long,
        visible_alias = "enable-udp-puncture",
        value_name = "BOOL",
        default_value_t = false,
        hide = true
    )]
    pub udp: bool,

    /// Disable fallback to TCP.
    /// TCP won't be used to transfer data between the Inlet and the Outlet.
    #[arg(
        long,
        visible_alias = "disable-tcp-fallback",
        value_name = "BOOL",
        default_value_t = false,
        hide = true
    )]
    pub no_tcp_fallback: bool,

    /// Use eBPF and RawSocket to access TCP packets instead of TCP data stream.
    /// If `OCKAM_PRIVILEGED` env variable is set to 1, this argument will be `true`.
    /// WARNING: This flag value should be equal on both ends of a portal (inlet and outlet)
    #[arg(long, env = "OCKAM_PRIVILEGED", value_parser = FalseyValueParser::default(), hide = true)]
    pub privileged: bool,

    /// [DEPRECATED] Use the `tls` scheme in the `--from` argument.
    #[arg(long, value_name = "BOOL", default_value_t = false, hide = true)]
    pub tls: bool,

    /// Enable TLS for the TCP Inlet using the provided certificate provider.
    /// Requires `ockam-tls-certificate` credential attribute.
    #[arg(long, value_name = "ROUTE", hide = true)]
    pub tls_certificate_provider: Option<MultiAddr>,

    /// Skip Portal handshake for lower latency, but also lower throughput
    /// WARNING: This flag value should be equal on both ends of a portal (inlet and outlet)
    #[arg(long, env = "OCKAM_TCP_PORTAL_SKIP_HANDSHAKE", value_parser = FalseyValueParser::default())]
    pub skip_handshake: bool,

    /// Enable Nagle's algorithm for potentially higher throughput, but higher latency
    #[arg(long, env = "OCKAM_TCP_PORTAL_ENABLE_NAGLE", value_parser = FalseyValueParser::default())]
    pub enable_nagle: bool,

    /// Enable MPTCP support
    #[arg(long, env = "OCKAM_TCP_PORTAL_ENABLE_MPTCP", value_parser = FalseyValueParser::default())]
    pub enable_mptcp: bool,

    #[arg(long, value_name = "HTTP_HEADER", value_parser = http_header_parser)]
    /// Set the provided HTTP headers in the client request. Existing headers with the same name
    /// will be discarded. This option assumes the protocol is HTTP/1.0 or HTTP/1.1.
    /// It expects a key-value pair in the format `key:value`. It can be specified multiple times.
    pub http_header: Vec<(String, String)>,
}

pub(crate) fn tcp_inlet_default_from_addr() -> SchemeHostnamePort {
    SchemeHostnamePort::from_str("127.0.0.1:0").unwrap()
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "tcp-inlet create";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        initialize_default_node(ctx, &opts).await?;
        let cmd = self.parse_args(&opts).await?;

        let mut node = BackgroundNodeClient::create(ctx, opts.state.clone(), &cmd.at).await?;
        cmd.timeout.timeout.map(|t| node.set_timeout_mut(t));

        let inlet_status = {
            let pb = opts.terminal.spinner();

            let prefix_route = if !cmd.http_header.is_empty() {
                let overwrite_http_header_address = Address::random_tagged("http_interceptor");

                if let Some(pb) = pb.as_ref() {
                    pb.set_message(format!(
                        "Creating HTTP Interceptor Service at {}...\n",
                        color_primary(&overwrite_http_header_address)
                    ));
                }

                let result = node
                    .create_http_header_overwrite_service(
                        ctx,
                        &overwrite_http_header_address,
                        cmd.http_header.clone(),
                    )
                    .await;

                match result {
                    Ok(_) => {
                        if let Some(pb) = pb.as_ref() {
                            let created_message = format!(
                                "Created a new HTTP Interceptor Service bound to {}\n",
                                color_primary(overwrite_http_header_address.to_string()),
                            );
                            pb.set_message(fmt_ok!("{}", created_message));
                        }
                    }
                    Err(_) => Err(miette!("Failed to create interceptor"))?,
                }

                route![overwrite_http_header_address]
            } else {
                route![]
            };

            if let Some(pb) = pb.as_ref() {
                pb.set_message(format!(
                    "Creating TCP Inlet at {}...\n",
                    color_primary(cmd.from.to_string())
                ));
            }

            loop {
                let result: Reply<InletStatus> = node
                    .create_inlet(
                        ctx,
                        cmd.from.hostname_port(),
                        &cmd.to(),
                        cmd.name.as_ref().expect("The `name` argument should be set to its default value if not provided"),
                        &cmd.authorized,
                        &cmd.allow,
                        cmd.connection_wait,
                        !cmd.no_connection_wait,
                        &cmd.secure_channel_identifier(opts.state.clone()).await?,
                        cmd.udp || cmd.from.is_udp(),
                        cmd.no_tcp_fallback,
                        cmd.privileged,
                        &cmd.tls_certificate_provider,
                        cmd.skip_handshake,
                        cmd.enable_nagle,
                        cmd.enable_mptcp,
                        prefix_route.clone(),
                    )
                    .await?;

                match result {
                    Reply::Successful(inlet_status) => {
                        break inlet_status;
                    }
                    Reply::Failed(_, s) => {
                        if let Some(status) = s {
                            if status == Status::BadRequest {
                                Err(miette!("Bad request when creating an inlet"))?
                            }
                        };
                        trace!("the inlet creation returned a non-OK status: {s:?}");

                        if cmd.retry_wait.as_millis() == 0 {
                            return Err(miette!("Failed to create TCP inlet"))?;
                        }

                        if let Some(pb) = pb.as_ref() {
                            pb.set_message(format!(
                                "Waiting for TCP Inlet {} to be available... Retrying momentarily\n",
                                color_primary(&cmd.to)
                            ));
                        }
                        tokio::time::sleep(cmd.retry_wait).await
                    }
                }
            }
        };

        let node_name = node.node_name();
        cmd.add_inlet_created_event(&opts, node_name, &inlet_status)
            .await?;

        let created_message = format!(
            "Created a new TCP Inlet in the Node {} bound to {}",
            color_primary(node_name),
            color_primary(inlet_status.bind_addr.to_string()),
        );

        let mut plain = if cmd.no_connection_wait {
            fmt_ok!("{created_message}\n")
                + &fmt_info!(
                    "It will automatically connect to the TCP Outlet at {} as soon as it is available\n",
                    color_primary(&cmd.to)
                )
        } else if inlet_status.status == ConnectionStatus::Up {
            fmt_ok!("{created_message}\n")
                + &fmt_log!(
                    "sending traffic to the TCP Outlet at {}\n",
                    color_primary(&cmd.to)
                )
        } else {
            fmt_warn!("{created_message}\n")
                + &fmt_log!(
                    "but it failed to connect to the TCP Outlet at {}\n",
                    color_primary(&cmd.to)
                )
                + &fmt_info!(
                    "It will automatically connect to the TCP Outlet as soon as it is available\n",
                )
        };

        if cmd.privileged {
            plain += &fmt_info!(
                "This TCP Inlet is operating in {} mode\n",
                color_primary_alt("privileged".to_string())
            );
        }

        opts.terminal
            .to_stdout()
            .plain(plain)
            .machine(inlet_status.bind_addr.to_string())
            .json(serde_json::json!(&inlet_status))
            .write_line()?;

        Ok(())
    }
}

impl CreateCommand {
    pub fn to(&self) -> MultiAddr {
        MultiAddr::from_str(&self.to).unwrap()
    }

    pub async fn secure_channel_identifier(
        &self,
        state: Arc<CliState>,
    ) -> miette::Result<Option<Identifier>> {
        if let Some(identity_name) = self.identity.as_ref() {
            Ok(Some(state.get_identifier_by_name(identity_name).await?))
        } else {
            Ok(None)
        }
    }

    pub async fn add_inlet_created_event(
        &self,
        opts: &CommandGlobalOpts,
        node_name: &str,
        inlet: &InletStatus,
    ) -> miette::Result<()> {
        let mut attributes = HashMap::new();
        attributes.insert(TCP_INLET_AT, node_name.to_string());
        attributes.insert(TCP_INLET_FROM, self.from.to_string());
        attributes.insert(TCP_INLET_TO, self.to.clone());
        attributes.insert(TCP_INLET_ALIAS, inlet.alias.clone());
        attributes.insert(TCP_INLET_CONNECTION_STATUS, inlet.status.to_string());
        attributes.insert(NODE_NAME, node_name.to_string());
        Ok(opts
            .state
            .add_journey_event(JourneyEvent::TcpInletCreated, attributes)
            .await?)
    }

    pub async fn parse_args(mut self, opts: &CommandGlobalOpts) -> miette::Result<Self> {
        if let Some(alias) = self.alias.as_ref() {
            print_warning_for_deprecated_flag_replaced(
                opts,
                "alias",
                "the <NAME> positional argument",
            )?;
            if self.name.is_some() {
                opts.terminal.write_line(
                    fmt_warn!("The <NAME> argument is being overridden by the --alias flag")
                        + &fmt_log!("Consider removing the --alias flag"),
                )?;
            }
            self.name = Some(alias.clone());
        } else {
            self.name = self.name.or_else(|| Some(random_name()));
        }

        let from = resolve_peer(self.from.hostname_port())
            .await
            .into_diagnostic()?;
        port_is_free_guard(&from)?;

        self.to = parse_to_address(&opts.state, self.to, self.via.as_ref()).await?;
        if self.to().matches(0, &[proto::Project::CODE.into()]) && self.authorized.is_some() {
            return Err(miette!(
                "--authorized can not be used with project addresses"
            ))?;
        }

        self.tls_certificate_provider =
            if let Some(tls_certificate_provider) = &self.tls_certificate_provider {
                Some(tls_certificate_provider.clone())
            } else if self.tls || self.from.is_tls() {
                Some(MultiAddr::from_str(
                    "/project/default/service/tls_certificate_provider",
                )?)
            } else {
                None
            };

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::run::parser::resource::utils::parse_cmd_from_args;

    use super::*;

    #[test]
    fn command_can_be_parsed_from_name() {
        let cmd = parse_cmd_from_args(CreateCommand::NAME, &[]);
        assert!(cmd.is_ok());
    }
}
