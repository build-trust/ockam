use std::collections::HashMap;
use std::str::FromStr;

use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use opentelemetry::trace::FutureExt;
use tokio::sync::Mutex;
use tokio::try_join;

use crate::node::util::initialize_default_node;
use crate::{docs, Command, CommandGlobalOpts};
use ockam::Context;
use ockam_abac::Expr;
use ockam_api::address::extract_address_value;
use ockam_api::cli_state::journeys::{
    JourneyEvent, NODE_NAME, TCP_OUTLET_AT, TCP_OUTLET_FROM, TCP_OUTLET_TO,
};
use ockam_api::colors::color_primary;
use ockam_api::nodes::service::portals::Outlets;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{fmt_info, fmt_log, fmt_ok};
use ockam_core::Address;
use ockam_transport_tcp::HostnamePort;

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

    /// If tls is set then the outlet will establish a TLS connection over TCP
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
    #[arg(hide = true, long = "allow", display_order = 904, id = "EXPRESSION")]
    pub policy_expression: Option<Expr>,
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "tcp-outlet create";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        initialize_default_node(ctx, &opts).await?;
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.at).await?;
        let node_name = node.node_name();
        let is_finished: Mutex<bool> = Mutex::new(false);

        let send_req = async {
            let from = self.from.map(Address::from);
            let res = node
                .create_outlet(
                    ctx,
                    self.to.clone(),
                    self.tls,
                    from.as_ref(),
                    self.policy_expression,
                )
                .await?;
            *is_finished.lock().await = true;
            Ok(res)
        }
        .with_current_context();

        let output_messages = vec![
            format!(
                "Attempting to create TCP Outlet to {}...",
                color_primary(self.to.to_string())
            ),
            format!(
                "Creating outlet service on node {}...",
                color_primary(&node_name)
            ),
            "Setting up TCP outlet worker...".to_string(),
        ];

        // Display progress spinner.
        let progress_output = opts
            .terminal
            .progress_output(&output_messages, &is_finished);

        let (outlet_status, _) = try_join!(send_req, progress_output)?;
        let worker_addr = outlet_status.worker_address().into_diagnostic()?;
        let json = serde_json::to_string_pretty(&outlet_status).into_diagnostic()?;

        let mut attributes = HashMap::new();
        attributes.insert(TCP_OUTLET_AT, node_name.clone());
        attributes.insert(TCP_OUTLET_FROM, worker_addr.to_string().clone());
        attributes.insert(TCP_OUTLET_TO, self.to.to_string());
        attributes.insert(NODE_NAME, node_name.clone());
        opts.state
            .add_journey_event(JourneyEvent::TcpOutletCreated, attributes)
            .await?;

        opts.terminal
            .stdout()
            .plain(
                fmt_ok!("Created a new TCP Outlet\n")
                    + &fmt_log!("  Node: {}\n", color_primary(&node_name))
                    + &fmt_log!(
                        "  Outlet Address: {}\n",
                        color_primary(outlet_status.worker_addr.address())
                    )
                    + &fmt_log!("  Socket Address: {}\n", color_primary(self.to.to_string()))
                    + &fmt_info!(
                        "You may want to take a look at the {}, {}, {} commands next",
                        color_primary("ockam relay"),
                        color_primary("ockam tcp-inlet"),
                        color_primary("ockam policy")
                    ),
            )
            .machine(worker_addr)
            .json(json)
            .write_line()?;

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
