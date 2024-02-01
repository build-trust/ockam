use std::collections::HashMap;
use std::net::SocketAddr;

use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use opentelemetry::trace::FutureExt;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_abac::Expr;
use ockam_api::journeys::{
    JourneyEvent, NODE_NAME, TCP_OUTLET_ALIAS, TCP_OUTLET_AT, TCP_OUTLET_FROM, TCP_OUTLET_TO,
};
use ockam_api::nodes::service::portals::Outlets;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{address::extract_address_value, random_name};

use crate::node::util::initialize_default_node;
use crate::tcp::util::alias_parser;
use crate::util::async_cmd;
use crate::util::parsers::socket_addr_parser;
use crate::{docs, fmt_info, fmt_ok, CommandGlobalOpts};
use crate::{fmt_log, terminal::color_primary};

const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");
const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");

/// Create a TCP Outlet that runs adjacent to a TCP server
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    /// TCP address where your TCP server is running. Your Outlet will send raw TCP traffic to it
    #[arg(long, display_order = 900, id = "SOCKET_ADDRESS", value_parser = socket_addr_parser)]
    pub to: SocketAddr,

    /// Assign a name to your Outlet. This name must be unique. If you don't provide it, a
    /// random name will be generated. This alias name is the resource name that you will
    /// need if you declare an access control policy for this Outlet using `ockam policy
    /// create`
    #[arg(long, display_order = 901, id = "ALIAS", default_value_t = random_name(), value_parser = alias_parser)]
    pub alias: String,

    /// Address of your TCP Outlet, which is part of a route that is used in other
    /// commands. This address must be unique. This address identifies the TCP Outlet
    /// worker, on the node, on your local machine. Examples are `/service/my-outlet` or
    /// `my-outlet`. If you don't provide it, `/service/outlet` will be used. You will
    /// need this address when you create a TCP Inlet (using `ockam tcp-inlet create --to
    /// <OUTLET_ADDRESS>`)
    #[arg(long, display_order = 902, id = "OUTLET_ADDRESS", default_value = "/service/outlet", value_parser = extract_address_value)]
    pub from: String,

    /// Your TCP Outlet will be created on this node. If you don't provide it, the default
    /// node will be used
    #[arg(long, display_order = 903, id = "NODE_NAME", value_parser = extract_address_value)]
    pub at: Option<String>,

    #[arg(hide = true, long = "policy", display_order = 904, id = "EXPRESSION")]
    pub policy_expression: Option<Expr>,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "create tcp outlet".into()
    }

    pub async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        initialize_default_node(ctx, &opts).await?;
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.at).await?;
        let node_name = node.node_name();
        let is_finished: Mutex<bool> = Mutex::new(false);

        let send_req = async {
            let node = BackgroundNodeClient::create(ctx, &opts.state, &self.at).await?;
            let res = node
                .create_outlet(
                    ctx,
                    &self.to,
                    &self.from.clone().into(),
                    self.alias,
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
            format!("Hosting outlet service at {}...", color_primary(&self.from)),
        ];

        // Display progress spinner.
        let progress_output = opts
            .terminal
            .progress_output(&output_messages, &is_finished);

        let (outlet_status, _) = try_join!(send_req, progress_output)?;
        let machine = outlet_status.worker_address().into_diagnostic()?;
        let json = serde_json::to_string_pretty(&outlet_status).into_diagnostic()?;

        let mut attributes = HashMap::new();
        attributes.insert(TCP_OUTLET_AT, node_name.clone());
        attributes.insert(TCP_OUTLET_FROM, self.from.clone());
        attributes.insert(TCP_OUTLET_TO, self.to.to_string());
        attributes.insert(TCP_OUTLET_ALIAS, outlet_status.alias.clone());
        attributes.insert(NODE_NAME, node_name.clone());
        opts.state
            .add_journey_event(JourneyEvent::TcpOutletCreated, attributes)
            .await?;

        opts.terminal
            .stdout()
            .plain(
                fmt_ok!("Created a new TCP Outlet\n")
                    + &fmt_log!("  Alias: {}\n", color_primary(outlet_status.alias.as_str()))
                    + &fmt_log!("  Node: {}\n", color_primary(&node_name))
                    + &fmt_log!("  Outlet Address: {}\n", color_primary(self.from))
                    + &fmt_log!("  Socket Address: {}\n", color_primary(self.to.to_string()))
                    + &fmt_info!(
                        "You may want to take a look at the {}, {}, {} commands next",
                        color_primary("ockam relay"),
                        color_primary("ockam tcp-inlet"),
                        color_primary("ockam policy")
                    ),
            )
            .machine(machine)
            .json(json)
            .write_line()?;

        Ok(())
    }
}
