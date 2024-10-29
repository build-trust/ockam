use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use async_trait::async_trait;
use clap::builder::FalseyValueParser;
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use tracing::trace;

use crate::node::util::initialize_default_node;
use crate::shared_args::OptionalTimeoutArg;
use crate::tcp::util::alias_parser;
use crate::{docs, Command, CommandGlobalOpts, Error};
use ockam::identity::Identifier;
use ockam::transport::HostnamePort;
use ockam::Context;
use ockam_abac::PolicyExpression;
use ockam_api::address::extract_address_value;
use ockam_api::cli_state::journeys::{
    JourneyEvent, NODE_NAME, TCP_INLET_ALIAS, TCP_INLET_AT, TCP_INLET_CONNECTION_STATUS,
    TCP_INLET_FROM, TCP_INLET_TO,
};
use ockam_api::cli_state::{random_name, CliState};
use ockam_api::colors::color_primary;
use ockam_api::nodes::models::portal::InletStatus;
use ockam_api::nodes::service::tcp_inlets::Inlets;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{fmt_info, fmt_log, fmt_ok, fmt_warn, ConnectionStatus};
use ockam_core::api::{Reply, Status};
use ockam_multiaddr::proto;
use ockam_multiaddr::{MultiAddr, Protocol as _};
use ockam_node::compat::asynchronous::resolve_peer;

use crate::util::parsers::duration_parser;
use crate::util::parsers::hostname_parser;
use crate::util::{find_available_port, port_is_free_guard, process_nodes_multiaddr};

const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create TCP Inlets
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct CreateCommand {
    /// Node on which to start the TCP Inlet.
    #[arg(long, display_order = 900, id = "NODE_NAME", value_parser = extract_address_value)]
    pub at: Option<String>,

    /// Address on which to accept TCP connections.
    #[arg(long, display_order = 900, id = "SOCKET_ADDRESS", hide_default_value = true, default_value_t = default_from_addr(), value_parser = hostname_parser)]
    pub from: HostnamePort,

    /// Route to a TCP Outlet or the name of the TCP Outlet service you want to connect to.
    ///
    /// If you are connecting to a local node, you can provide the route as `/node/n/service/outlet`.
    ///
    /// If you are connecting to a remote node through a relay in the Orchestrator you can either
    /// provide the full route to the TCP Outlet as `/project/myproject/service/forward_to_myrelay/secure/api/service/outlet`,
    /// or just the name of the service as `outlet` or `/service/outlet`.
    /// If you are passing just the service name, consider using `--via` to specify the
    /// relay name (e.g. `ockam tcp-inlet create --to outlet --via myrelay`).
    #[arg(long, display_order = 900, id = "ROUTE", default_value_t = default_to_addr())]
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

    /// Authorized identifier for secure channel connection
    #[arg(long, name = "AUTHORIZED", display_order = 900)]
    pub authorized: Option<Identifier>,

    /// Assign a name to this TCP Inlet.
    #[arg(long, display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    pub alias: Option<String>,

    /// Policy expression that will be used for access control to the TCP Inlet.
    /// If you don't provide it, the policy set for the "tcp-inlet" resource type will be used.
    ///
    /// You can check the fallback policy with `ockam policy show --resource-type tcp-inlet`.
    #[arg(
        hide = true,
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

    /// Enable UDP NAT puncture.
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
    /// If `OCKAM_EBPF` env variable is set to 1, this argument will be `true`.
    #[arg(long, env = "OCKAM_EBPF", value_parser = FalseyValueParser::default(), hide = true)]
    pub ebpf: bool,

    #[arg(long, value_name = "BOOL", default_value_t = false, hide = true)]
    /// Enable TLS for the TCP Inlet.
    /// Uses the default project TLS certificate provider, `/project/default/service/tls_certificate_provider`.
    /// To specify a different certificate provider, use `--tls-certificate-provider`.
    /// Requires `ockam-tls-certificate` credential attribute.
    pub tls: bool,

    #[arg(long, value_name = "ROUTE", hide = true)]
    /// Enable TLS for the TCP Inlet using the provided certificate provider.
    /// Requires `ockam-tls-certificate` credential attribute.
    pub tls_certificate_provider: Option<MultiAddr>,
}

pub(crate) fn default_from_addr() -> HostnamePort {
    let port = find_available_port().expect("Failed to find available port");
    HostnamePort::new("127.0.0.1", port)
}

fn default_to_addr() -> String {
    "/project/<default_project_name>/service/forward_to_<default_relay_name>/secure/api/service/<default_service_name>".to_string()
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "tcp-inlet create";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        initialize_default_node(ctx, &opts).await?;

        let cmd = self.parse_args(&opts).await?;

        let mut node = BackgroundNodeClient::create(ctx, &opts.state, &cmd.at).await?;
        cmd.timeout.timeout.map(|t| node.set_timeout_mut(t));

        let inlet_status = {
            let pb = opts.terminal.progress_bar();
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
                        &cmd.from,
                        &cmd.to(),
                        cmd.alias.as_ref().expect("The `alias` argument should be set to its default value if not provided"),
                        &cmd.authorized,
                        &cmd.allow,
                        cmd.connection_wait,
                        !cmd.no_connection_wait,
                        &cmd.secure_channel_identifier(&opts.state).await?,
                        cmd.udp,
                        cmd.no_tcp_fallback,
                        cmd.ebpf,
                        &cmd.tls_certificate_provider,
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
        cmd.add_inlet_created_event(&opts, &node_name, &inlet_status)
            .await?;

        let created_message = fmt_ok!(
            "Created a new TCP Inlet in the Node {} bound to {}\n",
            color_primary(&node_name),
            color_primary(cmd.from.to_string())
        );

        let plain = if cmd.no_connection_wait {
            created_message + &fmt_log!("It will automatically connect to the TCP Outlet at {} as soon as it is available",
                color_primary(&cmd.to)
            )
        } else if inlet_status.status == ConnectionStatus::Up {
            created_message
                + &fmt_log!(
                    "sending traffic to the TCP Outlet at {}",
                    color_primary(&cmd.to)
                )
        } else {
            fmt_warn!(
                "A TCP Inlet was created in the Node {} bound to {} but failed to connect to the TCP Outlet at {}\n",
                color_primary(&node_name),
                 color_primary(cmd.from.to_string()),
                color_primary(&cmd.to)
            ) + &fmt_info!("It will retry to connect automatically")
        };

        opts.terminal
            .stdout()
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
        state: &CliState,
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
        self.alias = self.alias.or_else(|| Some(random_name()));
        let from = resolve_peer(self.from.to_string())
            .await
            .into_diagnostic()?;
        port_is_free_guard(&from)?;
        self.to = Self::parse_arg_to(&opts.state, self.to, self.via.as_ref()).await?;
        if self.to().matches(0, &[proto::Project::CODE.into()]) && self.authorized.is_some() {
            return Err(miette!(
                "--authorized can not be used with project addresses"
            ))?;
        }
        self.tls_certificate_provider = if let Some(tls_certificate_provider) =
            &self.tls_certificate_provider
        {
            Some(tls_certificate_provider.clone())
        } else if self.tls {
            Some(MultiAddr::from_str("/project/default/service/tls_certificate_provider").unwrap())
        } else {
            None
        };
        Ok(self)
    }

    async fn parse_arg_to(
        state: &CliState,
        to: impl Into<String>,
        via: Option<&String>,
    ) -> miette::Result<String> {
        let mut to = to.into();
        let to_is_default = to == default_to_addr();
        let mut service_name = "outlet".to_string();
        let relay_name = via.cloned().unwrap_or("default".to_string());

        match MultiAddr::from_str(&to) {
            // "to" is a valid multiaddr
            Ok(to) => {
                // check whether it's a full route or a single service
                if let Some(proto) = to.first() {
                    // "to" refers to the service name
                    if proto.code() == proto::Service::CODE && to.len() == 1 {
                        service_name = proto
                            .cast::<proto::Service>()
                            .ok_or_else(|| Error::arg_validation("to", via, None))?
                            .to_string();
                    }
                    // "to" is a full route
                    else {
                        // "via" can't be passed if the user provides a value for "to"
                        if !to_is_default && via.is_some() {
                            return Err(Error::arg_validation(
                                "to",
                                via,
                                Some("'via' can't be passed if 'to' is a route"),
                            ))?;
                        }
                    }
                }
            }
            // If it's not
            Err(_) => {
                // "to" refers to the service name
                service_name = to.to_string();
                // and we set "to" to the default route, so we can do the replacements later
                to = default_to_addr();
            }
        }

        // Replace the placeholders
        if to.contains("<default_project_name>") {
            let project_name = state
                .projects()
                .get_default_project()
                .await
                .map(|p| p.name().to_string())
                .ok()
                .ok_or(Error::arg_validation("to", via, Some("No projects found")))?;
            to = to.replace("<default_project_name>", &project_name);
        }
        to = to.replace("<default_relay_name>", &relay_name);
        to = to.replace("<default_service_name>", &service_name);

        // Parse "to" as a multiaddr again with all the values in place
        let to = MultiAddr::from_str(&to).into_diagnostic()?;
        Ok(process_nodes_multiaddr(&to, state).await?.to_string())
    }
}

#[cfg(test)]
mod tests {
    use ockam_api::cloud::project::models::ProjectModel;
    use ockam_api::cloud::project::Project;
    use ockam_api::nodes::InMemoryNode;

    use crate::run::parser::resource::utils::parse_cmd_from_args;

    use super::*;

    #[test]
    fn command_can_be_parsed_from_name() {
        let cmd = parse_cmd_from_args(CreateCommand::NAME, &[]);
        assert!(cmd.is_ok());
    }

    #[ockam_macros::test]
    async fn parse_arg_to(ctx: &mut Context) -> ockam_core::Result<()> {
        // Setup
        let state = CliState::test().await.unwrap();
        let node = InMemoryNode::start(ctx, &state).await.unwrap();
        let node_name = node.node_name();
        let node_port = state
            .get_node(&node_name)
            .await
            .unwrap()
            .tcp_listener_port()
            .unwrap();
        let project = Project::import(ProjectModel {
            identity: Some(
                Identifier::from_str(
                    "Ie92f183eb4c324804ef4d62962dea94cf095a265a1b2c3d4e5f6a6b5c4d3e2f1",
                )
                .unwrap(),
            ),
            name: "p1".to_string(),
            ..Default::default()
        })
        .await
        .unwrap();
        state.projects().store_project(project).await.unwrap();

        // Invalid "to" values throw an error
        let cases = ["/alice/service", "alice/relay"];
        for to in cases {
            CreateCommand::parse_arg_to(&state, to, None)
                .await
                .expect_err("Invalid multiaddr");
        }

        // "to" default value
        let res = CreateCommand::parse_arg_to(&state, default_to_addr(), None)
            .await
            .unwrap();
        assert_eq!(
            res,
            "/project/p1/service/forward_to_default/secure/api/service/outlet".to_string()
        );

        // "to" argument accepts a full route
        let cases = [
            ("/project/p2/service/forward_to_n1/secure/api/service/myoutlet", None),
            ("/worker/603b62d245c9119d584ba3d874eb8108/service/forward_to_n3/service/hop/service/outlet", None),
            (&format!("/node/{node_name}/service/myoutlet"), Some(format!("/ip4/127.0.0.1/tcp/{node_port}/service/myoutlet"))),
        ];
        for (to, expected) in cases {
            let res = CreateCommand::parse_arg_to(&state, to, None).await.unwrap();
            let expected = expected.unwrap_or(to.to_string());
            assert_eq!(res, expected);
        }

        // "to" argument accepts the name of the service
        let res = CreateCommand::parse_arg_to(&state, "myoutlet", None)
            .await
            .unwrap();
        assert_eq!(
            res,
            "/project/p1/service/forward_to_default/secure/api/service/myoutlet".to_string()
        );

        // "via" argument is used to replace the relay name
        let cases = [
            (
                default_to_addr(),
                "myrelay",
                "/project/p1/service/forward_to_myrelay/secure/api/service/outlet",
            ),
            (
                "myoutlet".to_string(),
                "myrelay",
                "/project/p1/service/forward_to_myrelay/secure/api/service/myoutlet",
            ),
        ];
        for (to, via, expected) in cases {
            let res = CreateCommand::parse_arg_to(&state, &to, Some(&via.to_string()))
                .await
                .unwrap();
            assert_eq!(res, expected.to_string());
        }

        // if "to" is passed as a full route and also "via" is passed, return an error
        let to = "/project/p1/service/forward_to_n1/secure/api/service/outlet";
        CreateCommand::parse_arg_to(&state, to, Some(&"myrelay".to_string()))
            .await
            .expect_err("'via' can't be passed if 'to' is a full route");

        Ok(())
    }
}
