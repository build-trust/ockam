use std::collections::HashMap;
use std::net::SocketAddr;

use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use opentelemetry::trace::FutureExt;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_abac::Resource;
use ockam_api::address::extract_address_value;
use ockam_api::journeys::{
    JourneyEvent, NODE_NAME, TCP_OUTLET_ALIAS, TCP_OUTLET_AT, TCP_OUTLET_FROM, TCP_OUTLET_TO,
};
use ockam_api::nodes::models::portal::{CreateOutlet, OutletStatus};
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;

use crate::fmt_log;
use crate::node::util::initialize_default_node;
use crate::policy::{add_default_project_policy, has_policy};
use crate::tcp::util::alias_parser;
use crate::terminal::OckamColor;
use crate::util::async_cmd;
use crate::util::parsers::socket_addr_parser;
use crate::{docs, fmt_ok, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a TCP Outlet
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct CreateCommand {
    /// Node on which to start the tcp outlet.
    #[arg(long, display_order = 900, id = "NODE_NAME", value_parser = extract_address_value)]
    pub at: Option<String>,

    /// Address of the tcp outlet.
    #[arg(long, display_order = 901, id = "OUTLET_ADDRESS", default_value_t = default_from_addr(), value_parser = extract_address_value)]
    pub from: String,

    /// TCP address to send raw tcp traffic.
    #[arg(long, display_order = 902, id = "SOCKET_ADDRESS", value_parser = socket_addr_parser)]
    pub to: SocketAddr,

    /// Assign a name to this outlet.
    #[arg(long, display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    pub alias: Option<String>,
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
        opts.terminal.write_line(&fmt_log!(
            "Creating TCP Outlet to {}...\n",
            &self
                .to
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ))?;

        let node_name = opts.state.get_node_or_default(&self.at).await?.name();
        let project = opts.state.get_node_project(&node_name).await.ok();
        let resource = Resource::new("tcp-outlet");
        if project.is_some() && !has_policy(&node_name, ctx, &opts, &resource).await? {
            add_default_project_policy(&node_name, ctx, &opts, &resource).await?;
        }

        let is_finished: Mutex<bool> = Mutex::new(false);

        let send_req = async {
            let payload = CreateOutlet::new(self.to, self.from.clone().into(), self.alias, true);
            let res = send_request(ctx, &opts, payload, node_name.clone()).await;
            *is_finished.lock().await = true;
            res
        }
        .with_current_context();

        let output_messages = vec![
            format!(
                "Creating outlet service on node {}...",
                &node_name
                    .to_string()
                    .color(OckamColor::PrimaryResource.color()),
            ),
            "Setting up TCP outlet worker...".to_string(),
            format!(
                "Hosting outlet service at {}...",
                self.from.clone().color(OckamColor::PrimaryResource.color())
            ),
        ];

        let progress_output = opts
            .terminal
            .progress_output(&output_messages, &is_finished);

        let (outlet_status, _) = try_join!(send_req, progress_output)?;
        let machine = outlet_status.worker_address().into_diagnostic()?;
        let json = serde_json::to_string_pretty(&outlet_status).into_diagnostic()?;

        let mut attributes = HashMap::default();
        let to = self.to.to_string();
        attributes.insert(TCP_OUTLET_AT, node_name.as_str());
        attributes.insert(TCP_OUTLET_FROM, self.from.as_str());
        attributes.insert(TCP_OUTLET_TO, to.as_str());
        attributes.insert(TCP_OUTLET_ALIAS, outlet_status.alias.as_str());
        attributes.insert(NODE_NAME, node_name.as_str());
        opts.state
            .add_journey_event(JourneyEvent::TcpOutletCreated, attributes)
            .await
            .unwrap();

        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "Created a new TCP Outlet on node {} from address {} to {}",
                &node_name
                    .to_string()
                    .color(OckamColor::PrimaryResource.color()),
                &self.from.color(OckamColor::PrimaryResource.color()),
                &self
                    .to
                    .to_string()
                    .color(OckamColor::PrimaryResource.color())
            ))
            .machine(machine)
            .json(json)
            .write_line()?;

        Ok(())
    }
}

pub fn default_from_addr() -> String {
    "/service/outlet".to_string()
}

pub async fn send_request(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    payload: CreateOutlet,
    to_node: impl Into<Option<String>>,
) -> crate::Result<OutletStatus> {
    let node = BackgroundNodeClient::create(ctx, &opts.state, &to_node.into()).await?;
    let req = Request::post("/node/outlet").body(payload);
    Ok(node.ask(ctx, req).await?)
}
