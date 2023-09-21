use std::net::SocketAddr;

use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_abac::Resource;
use ockam_api::address::extract_address_value;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::nodes::models::portal::{CreateOutlet, OutletStatus};
use ockam_api::nodes::RemoteNode;
use ockam_core::api::Request;

use crate::node::{get_node_name, initialize_node_if_default};
use crate::policy::{add_default_project_policy, has_policy};
use crate::tcp::util::alias_parser;
use crate::terminal::OckamColor;
use crate::util::node_rpc;
use crate::util::parsers::socket_addr_parser;
use crate::{display_parse_logs, fmt_log};
use crate::{docs, fmt_ok, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create TCP Outlets
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct CreateCommand {
    /// Node on which to start the tcp outlet.
    #[arg(long, display_order = 900, id = "NODE")]
    at: Option<String>,

    /// Address of the tcp outlet.
    #[arg(long, display_order = 901, id = "OUTLET_ADDRESS", default_value_t = default_from_addr())]
    from: String,

    /// TCP address to send raw tcp traffic.
    #[arg(long, display_order = 902, id = "SOCKET_ADDRESS", value_parser = socket_addr_parser)]
    to: SocketAddr,

    /// Assign a name to this outlet.
    #[arg(long, display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    alias: Option<String>,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.at);
        node_rpc(run_impl, (opts, self))
    }
}

pub fn default_from_addr() -> String {
    "/service/outlet".to_string()
}

pub async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> miette::Result<()> {
    opts.terminal.write_line(&fmt_log!(
        "Creating TCP Outlet to {}...\n",
        &cmd.to
            .to_string()
            .color(OckamColor::PrimaryResource.color())
    ))?;
    display_parse_logs(&opts);

    let node_name = get_node_name(&opts.state, &cmd.at);
    let node_name = extract_address_value(&node_name)?;
    let project = opts
        .state
        .nodes
        .get(&node_name)?
        .config()
        .setup()
        .project
        .to_owned();
    let resource = Resource::new("tcp-outlet");
    if let Some(p) = project {
        if !has_policy(&node_name, &ctx, &opts, &resource).await? {
            add_default_project_policy(&node_name, &ctx, &opts, p, &resource).await?;
        }
    }

    let is_finished: Mutex<bool> = Mutex::new(false);

    let send_req = async {
        let payload = CreateOutlet::new(
            cmd.to,
            extract_address_value(&cmd.from)?.into(),
            cmd.alias,
            true,
        );
        let res = send_request(&ctx, &opts, payload, node_name.clone()).await;
        *is_finished.lock().await = true;
        res
    };

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
            &cmd.from
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ),
    ];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (outlet_status, _) = try_join!(send_req, progress_output)?;
    let machine = outlet_status.worker_address().into_diagnostic()?;
    let json = serde_json::to_string_pretty(&outlet_status).into_diagnostic()?;

    opts.terminal
        .stdout()
        .plain(fmt_ok!(
            "Created a new TCP Outlet on node {} from address {} to {}",
            &node_name
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            format!("/service/{}", extract_address_value(&cmd.from)?)
                .color(OckamColor::PrimaryResource.color()),
            &cmd.to
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ))
        .machine(machine)
        .json(json)
        .write_line()?;

    Ok(())
}

pub async fn send_request(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    payload: CreateOutlet,
    to_node: impl Into<Option<String>>,
) -> crate::Result<OutletStatus> {
    let node_name = get_node_name(&opts.state, &to_node.into());
    let node = RemoteNode::create(ctx, &opts.state, &node_name).await?;
    let req = Request::post("/node/outlet").body(payload);
    Ok(node.ask(ctx, req).await?)
}
