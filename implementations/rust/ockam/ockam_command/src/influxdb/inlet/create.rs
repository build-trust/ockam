use crate::node::util::initialize_default_node;
use crate::shared_args::OptionalTimeoutArg;
use crate::tcp::inlet::create::{tcp_inlet_default_from_addr, tcp_inlet_default_to_addr};
use crate::tcp::util::alias_parser;
use crate::util::parsers::duration_parser;
use crate::util::parsers::hostname_parser;
use crate::util::{port_is_free_guard, print_warning_for_deprecated_flag_replaced};
use crate::{Command, CommandGlobalOpts};
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
use ockam_api::cli_state::random_name;
use ockam_api::colors::color_primary;
use ockam_api::influxdb::{InfluxDBPortals, LeaseUsage};
use ockam_api::nodes::models::portal::InletStatus;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{fmt_info, fmt_log, fmt_ok, fmt_warn, CliState, ConnectionStatus};
use ockam_core::api::{Reply, Status};
use ockam_multiaddr::{proto, MultiAddr, Protocol};
use ockam_node::compat::asynchronous::resolve_peer;
use std::str::FromStr;
use std::time::Duration;
use tracing::trace;

/// Create InfluxDB Inlets
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Assign a name to this InfluxDB Inlet
    #[arg(id = "NAME", value_parser = alias_parser)]
    pub name: Option<String>,

    /// Node on which to start the InfluxDB Inlet.
    #[arg(long, display_order = 900, id = "NODE_NAME", value_parser = extract_address_value)]
    pub at: Option<String>,

    /// Address on which to accept InfluxDB connections, in the format `<scheme>://<hostname>:<port>`.
    /// At least the port must be provided. The default scheme is `tcp` and the default hostname is `127.0.0.1`.
    /// If the argument is not set, a random port will be used on the default address.
    ///
    /// To enable TLS, the `ockam-tls-certificate` credential attribute is required.
    /// It will use the default project TLS certificate provider `/project/default/service/tls_certificate_provider`.
    /// To specify a different certificate provider, use `--tls-certificate-provider`.
    #[arg(long, display_order = 900, id = "SOCKET_ADDRESS", hide_default_value = true, default_value_t = tcp_inlet_default_from_addr(), value_parser = hostname_parser)]
    pub from: SchemeHostnamePort,

    /// Route to a InfluxDB Outlet or the name of the InfluxDB Outlet service you want to connect to.
    ///
    /// If you are connecting to a local node, you can provide the route as `/node/n/service/outlet`.
    ///
    /// If you are connecting to a remote node through a relay in the Orchestrator you can either
    /// provide the full route to the InfluxDB Outlet as `/project/myproject/service/forward_to_myrelay/secure/api/service/outlet`,
    /// or just the name of the service as `outlet` or `/service/outlet`.
    /// If you are passing just the service name, consider using `--via` to specify the
    /// relay name (e.g. `ockam tcp-inlet create --to outlet --via myrelay`).
    #[arg(long, display_order = 900, id = "ROUTE", default_value_t = tcp_inlet_default_to_addr())]
    pub to: String,

    /// Name of the relay that this InfluxDB Inlet will use to connect to the InfluxDB Outlet.
    ///
    /// Use this flag when you are using `--to` to specify the service name of a InfluxDB Outlet
    /// that is reachable through a relay in the Orchestrator.
    /// If you don't provide it, the default relay name will be used, if necessary.
    #[arg(long, display_order = 900, id = "RELAY_NAME")]
    pub via: Option<String>,

    /// Identity to be used to create the secure channel. If not set, the node's identity will be used.
    #[arg(long, value_name = "IDENTITY_NAME", display_order = 900)]
    pub identity: Option<String>,

    /// Authorized identifier for secure channel connection
    #[arg(long, name = "AUTHORIZED", display_order = 900)]
    pub authorized: Option<Identifier>,

    /// [DEPRECATED] Use the <NAME> positional argument instead
    #[arg(long, display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    pub alias: Option<String>,

    /// Policy expression that will be used for access control to the InfluxDB Inlet.
    /// If you don't provide it, the policy set for the "tcp-inlet" resource type will be used.
    ///
    /// You can check the fallback policy with `ockam policy show --resource-type tcp-inlet`.
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

    /// Time to wait before retrying to connect to the InfluxDB Outlet.
    #[arg(long, display_order = 900, id = "RETRY", default_value = "20s", value_parser = duration_parser)]
    pub retry_wait: Duration,

    #[command(flatten)]
    pub timeout: OptionalTimeoutArg,

    /// Create the InfluxDB Inlet without waiting for the InfluxDB Outlet to connect
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
    #[arg(long, env = "OCKAM_PRIVILEGED", value_parser = FalseyValueParser::default(), hide = true)]
    pub privileged: bool,

    /// [DEPRECATED] Use the `tls` scheme in the `--from` argument.
    #[arg(long, value_name = "BOOL", default_value_t = false, hide = true)]
    pub tls: bool,

    /// Enable TLS for the InfluxDB Inlet using the provided certificate provider.
    /// Requires `ockam-tls-certificate` credential attribute.
    #[arg(long, value_name = "ROUTE", hide = true)]
    pub tls_certificate_provider: Option<MultiAddr>,

    /// Share the leases among the clients or use a separate lease for each client
    #[arg(long, default_value = "per-client")]
    pub leased_token_strategy: LeaseUsage,

    /// The route to the lease issuer service.
    /// Only applicable if `lease-token-strategy` is set to `per-client`.
    /// If not provided, it's derived from the outlet route.
    #[arg(long, value_name = "ROUTE")]
    pub lease_manager_route: Option<MultiAddr>,
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "influxdb-inlet create";

    async fn async_run(mut self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        initialize_default_node(ctx, &opts).await?;
        let cmd = self.parse_args(&opts).await?;

        let mut node = BackgroundNodeClient::create(ctx, &opts.state, &cmd.at).await?;
        cmd.timeout.timeout.map(|t| node.set_timeout_mut(t));

        let inlet_status = {
            let pb = opts.terminal.spinner();
            if let Some(pb) = pb.as_ref() {
                pb.set_message(format!(
                    "Creating a InfluxDB Inlet at {}...\n",
                    color_primary(&cmd.from)
                ));
            }

            loop {
                let result: Reply<InletStatus> = node
                    .create_influxdb_inlet(
                        ctx,
                        cmd.from.hostname_port(),
                        &cmd.to(),
                        cmd.name.as_ref().expect("The `name` argument should be set to its default value if not provided"),
                        &cmd.authorized,
                        &cmd.allow,
                        cmd.connection_wait,
                        !cmd.no_connection_wait,
                        &cmd
                            .secure_channel_identifier(&opts.state)
                            .await?,
                        cmd.udp || cmd.from.is_udp(),
                        cmd.no_tcp_fallback,
                        &cmd.tls_certificate_provider,
                        cmd.leased_token_strategy.clone(),
                        cmd.lease_manager_route.clone(),
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
                            return Err(miette!("Failed to create InfluxDB inlet"))?;
                        }

                        if let Some(pb) = pb.as_ref() {
                            pb.set_message(format!(
                                "Waiting for InfluxDB Inlet {} to be available... Retrying momentarily\n",
                                color_primary(&cmd.to)
                            ));
                        }
                        tokio::time::sleep(cmd.retry_wait).await
                    }
                }
            }
        };

        let node_name = node.node_name();

        let created_message = format!(
            "Created a new InfluxDB Inlet in the Node {} bound to {}",
            color_primary(&node_name),
            color_primary(cmd.from.to_string()),
        );

        let plain = if cmd.no_connection_wait {
            fmt_ok!("{created_message}\n")
                + &fmt_log!("It will automatically connect to the InfluxDB Outlet at {} as soon as it is available\n",
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
                "but failed to connect to the TCP Outlet at {}\n",
                color_primary(&cmd.to)
            ) + &fmt_info!("It will automatically connect to the InfluxDB Outlet as soon as it is available\n")
        };

        opts.terminal
            .stdout()
            .plain(plain)
            .machine(inlet_status.bind_addr.to_string())
            .json_obj(&inlet_status)?
            .write_line()?;

        Ok(())
    }
}

impl CreateCommand {
    async fn parse_args(mut self, opts: &CommandGlobalOpts) -> miette::Result<Self> {
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

        self.to = crate::tcp::inlet::create::CreateCommand::parse_arg_to(
            &opts.state,
            self.to,
            self.via.as_ref(),
        )
        .await?;
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

        if self
            .lease_manager_route
            .as_ref()
            .is_some_and(|_| self.leased_token_strategy == LeaseUsage::Shared)
        {
            Err(miette!(
                "lease-manager-route argument requires leased-token-strategy=per-client"
            ))?
        };

        Ok(self)
    }

    pub fn to(&self) -> MultiAddr {
        MultiAddr::from_str(&self.to).unwrap()
    }

    pub async fn secure_channel_identifier(
        &self,
        state: &CliState,
    ) -> miette::Result<Option<Identifier>> {
        if let Some(identity_name) = self.identity.as_ref() {
            Ok(Some(state.get_identifier_by_name(identity_name).await?))
        } else {
            Ok(None)
        }
    }
}
