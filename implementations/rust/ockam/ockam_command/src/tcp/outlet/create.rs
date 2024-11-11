use crate::node::util::initialize_default_node;
use crate::util::parsers::hostname_parser;
use crate::{docs, Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::builder::FalseyValueParser;
use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use ockam::transport::SchemeHostnamePort;
use ockam::Address;
use ockam::Context;
use ockam_abac::PolicyExpression;
use ockam_api::address::extract_address_value;
use ockam_api::cli_state::journeys::{
    JourneyEvent, NODE_NAME, TCP_OUTLET_AT, TCP_OUTLET_FROM, TCP_OUTLET_TO,
};
use ockam_api::colors::{color_primary, color_primary_alt};
use ockam_api::nodes::models::portal::OutletStatus;
use ockam_api::nodes::service::tcp_outlets::Outlets;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{fmt_info, fmt_log, fmt_ok, fmt_warn};
use std::collections::HashMap;

const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");
const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");

/// Create a TCP Outlet that runs adjacent to a TCP server
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    /// Address of your TCP Outlet, which is part of a route used in other commands.
    /// This unique address identifies the TCP Outlet worker on the Node on your local machine.
    /// Examples are `/service/my-outlet` or `my-outlet`.
    /// If not provided, `outlet` will be used, or a random address will be generated if `outlet` is taken.
    /// You will need this address when creating a TCP Inlet using `ockam tcp-inlet create`.
    #[arg(value_parser = extract_address_value)]
    pub name: Option<String>,

    /// TCP address where your TCP server is running: domain:port. Your Outlet will send raw TCP traffic to it
    #[arg(long, id = "SOCKET_ADDRESS", display_order = 900, value_parser = hostname_parser)]
    pub to: SchemeHostnamePort,

    /// If set, the outlet will establish a TLS connection over TCP
    #[arg(long, display_order = 900, id = "BOOLEAN")]
    pub tls: bool,

    /// Alternative to the <NAME> positional argument.
    /// Address of your TCP Outlet, which is part of a route used in other commands.
    #[arg(long, display_order = 902, id = "OUTLET_ADDRESS", value_parser = extract_address_value)]
    pub from: Option<String>,

    /// Your TCP Outlet will be created on this node. If you don't provide it, the default
    /// node will be used
    #[arg(long, display_order = 903, id = "NODE_NAME", value_parser = extract_address_value)]
    pub at: Option<String>,

    /// Policy expression that will be used for access control to the TCP Outlet.
    /// If you don't provide it, the policy set for the "tcp-outlet" resource type will be used.
    ///
    /// You can check the fallback policy with `ockam policy show --resource-type tcp-outlet`.
    #[arg(
        long,
        visible_alias = "expression",
        display_order = 904,
        id = "POLICY_EXPRESSION"
    )]
    pub allow: Option<PolicyExpression>,

    /// Use eBPF and RawSocket to access TCP packets instead of TCP data stream.
    /// If `OCKAM_PRIVILEGED` env variable is set to 1, this argument will be `true`.
    #[arg(long, env = "OCKAM_PRIVILEGED", value_parser = FalseyValueParser::default(), hide = true)]
    pub privileged: bool,
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "tcp-outlet create";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        initialize_default_node(ctx, &opts).await?;
        let cmd = self.parse_args(&opts).await?;

        let node = BackgroundNodeClient::create(ctx, &opts.state, &cmd.at).await?;
        let node_name = node.node_name();
        let outlet_status = {
            let pb = opts.terminal.spinner();
            if let Some(pb) = pb.as_ref() {
                pb.set_message(format!(
                    "Creating a new TCP Outlet to {}...\n",
                    color_primary(cmd.to.to_string())
                ));
            }
            node.create_outlet(
                ctx,
                cmd.to.clone().into(),
                cmd.tls,
                cmd.name.clone().map(Address::from).as_ref(),
                cmd.allow.clone(),
                cmd.privileged,
            )
            .await?
        };
        cmd.add_outlet_created_journey_event(&opts, &node_name, &outlet_status)
            .await?;

        let worker_route = outlet_status.worker_route().into_diagnostic()?;

        let mut msg = fmt_ok!(
            "Created a new TCP Outlet in the Node {} at {} bound to {}\n",
            color_primary(&node_name),
            color_primary(worker_route.to_string()),
            color_primary(cmd.to.to_string())
        );

        if cmd.privileged {
            msg += &fmt_info!(
                "This Outlet is operating in {} mode\n",
                color_primary_alt("privileged".to_string())
            );
        }

        opts.terminal
            .stdout()
            .plain(msg)
            .machine(worker_route)
            .json(serde_json::to_string(&outlet_status).into_diagnostic()?)
            .write_line()?;

        Ok(())
    }
}

impl CreateCommand {
    async fn parse_args(mut self, opts: &CommandGlobalOpts) -> miette::Result<Self> {
        if let Some(from) = self.from.as_ref() {
            if self.name.is_some() {
                opts.terminal.write_line(
                    fmt_warn!("The <NAME> argument is being overridden by the --from flag")
                        + &fmt_log!("Consider using either the <NAME> argument or the --from flag"),
                )?;
            }
            self.name = Some(from.clone());
        }

        Ok(self)
    }

    pub async fn add_outlet_created_journey_event(
        &self,
        opts: &CommandGlobalOpts,
        node_name: &str,
        outlet_status: &OutletStatus,
    ) -> miette::Result<()> {
        let mut attributes = HashMap::new();
        attributes.insert(TCP_OUTLET_AT, node_name.to_string());
        attributes.insert(
            TCP_OUTLET_FROM,
            outlet_status.worker_route().into_diagnostic()?.to_string(),
        );
        attributes.insert(TCP_OUTLET_TO, self.to.to_string());
        attributes.insert(NODE_NAME, node_name.to_string());
        opts.state
            .add_journey_event(JourneyEvent::TcpOutletCreated, attributes)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::run::parser::resource::utils::parse_cmd_from_args;

    use super::*;

    #[test]
    fn command_can_be_parsed_from_name() {
        let cmd = parse_cmd_from_args(
            CreateCommand::NAME,
            &["--to".to_string(), "127.0.0.1:5000".to_string()],
        );
        assert!(cmd.is_ok());
    }
}
