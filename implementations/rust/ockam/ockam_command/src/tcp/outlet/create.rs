use std::collections::HashMap;
use std::str::FromStr;

use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;

use crate::node::util::initialize_default_node;
use crate::{docs, Command, CommandGlobalOpts};
use ockam::Context;
use ockam_abac::PolicyExpression;
use ockam_api::address::extract_address_value;
use ockam_api::cli_state::journeys::{
    JourneyEvent, NODE_NAME, TCP_OUTLET_AT, TCP_OUTLET_FROM, TCP_OUTLET_TO,
};
use ockam_api::colors::color_primary;
use ockam_api::nodes::models::portal::OutletStatus;
use ockam_api::nodes::service::portals::Outlets;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{fmt_log, fmt_ok};
use ockam_core::Address;
use ockam_node::HostnamePort;

const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");
const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");

/// Create a TCP Outlet that runs adjacent to a TCP server
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    /// TCP address where your TCP server is running: domain:port. Your Outlet will send raw TCP traffic to it
    #[arg(long, display_order = 900, id = "HOSTNAME_PORT", value_parser = HostnamePort::from_str)]
    pub to: HostnamePort,

    /// If set, the outlet will establish a TLS connection over TCP
    #[arg(long, display_order = 900, id = "BOOLEAN")]
    pub tls: bool,

    /// Address of your TCP Outlet, which is part of a route that is used in other
    /// commands. This address must be unique. This address identifies the TCP Outlet
    /// worker, on the node, on your local machine. Examples are `/service/my-outlet` or
    /// `my-outlet`. If you don't provide it, `/service/outlet` will be used. You will
    /// need this address when you create a TCP Inlet (using `ockam tcp-inlet create --to
    /// <OUTLET_ADDRESS>`)
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
        hide = true,
        long,
        visible_alias = "expression",
        display_order = 904,
        id = "POLICY_EXPRESSION"
    )]
    pub allow: Option<PolicyExpression>,
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "tcp-outlet create";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        initialize_default_node(ctx, &opts).await?;

        if let Some(pb) = opts.terminal.progress_bar() {
            pb.set_message(format!(
                "Creating a new TCP Outlet to {}...\n",
                color_primary(self.to.to_string())
            ));
        }

        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.at).await?;
        let node_name = node.node_name();
        let outlet_status = node
            .create_outlet(
                ctx,
                self.to.clone(),
                self.tls,
                self.from.clone().map(Address::from).as_ref(),
                self.allow.clone(),
            )
            .await?;
        self.add_outlet_created_journey_event(&opts, &node_name, &outlet_status)
            .await?;

        let worker_addr = outlet_status.worker_address().into_diagnostic()?;

        opts.terminal
            .stdout()
            .plain(
                fmt_ok!(
                    "Created a new TCP Outlet in the Node {} at {} bound to {}\n\n",
                    color_primary(&node_name),
                    color_primary(worker_addr.to_string()),
                    color_primary(self.to.to_string())
                ) + &fmt_log!(
                    "You may want to take a look at the {}, {}, {} commands next",
                    color_primary("ockam relay"),
                    color_primary("ockam tcp-inlet"),
                    color_primary("ockam policy")
                ),
            )
            .machine(worker_addr)
            .json(serde_json::to_string(&outlet_status).into_diagnostic()?)
            .write_line()?;

        Ok(())
    }
}

impl CreateCommand {
    async fn add_outlet_created_journey_event(
        &self,
        opts: &CommandGlobalOpts,
        node_name: &str,
        outlet_status: &OutletStatus,
    ) -> miette::Result<()> {
        let mut attributes = HashMap::new();
        attributes.insert(TCP_OUTLET_AT, node_name.to_string());
        attributes.insert(
            TCP_OUTLET_FROM,
            outlet_status
                .worker_address()
                .into_diagnostic()?
                .to_string(),
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
