use crate::node::{default_node_name, node_name_parser};
use crate::policy::{add_default_project_policy, has_policy};
use crate::tcp::util::alias_parser;
use crate::terminal::OckamColor;

use crate::fmt_log;
use crate::util::parsers::socket_addr_parser;
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::{fmt_ok, CommandGlobalOpts};

use clap::Args;
use colorful::Colorful;
use ockam::Context;
use ockam_abac::Resource;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::nodes::models::portal::{CreateOutlet, OutletStatus};
use ockam_core::api::{Request, RequestBuilder};
use ockam_multiaddr::MultiAddr;
use std::net::SocketAddr;
use tokio::sync::Mutex;
use tokio::try_join;

/// Create TCP Outlets
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Node on which to start the tcp outlet.
    #[arg(long, display_order = 900, id = "NODE", default_value_t = default_node_name(), value_parser = node_name_parser)]
    at: String,

    /// Address of the tcp outlet.
    #[arg(long, display_order = 901, id = "OUTLET_ADDRESS", default_value_t = default_from_addr())]
    from: String,

    /// TCP address to send raw tcp traffic.
    #[arg(long, display_order = 902, id = "SOCKET_ADDRESS", value_parser = socket_addr_parser)]
    to: SocketAddr,

    /// Assign a name to this outlet.
    #[arg(long, display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    alias: Option<String>,

    #[arg(short, long, value_name = "EXPOSED")]
    pub exposed_to: Option<Vec<MultiAddr>>,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

fn default_from_addr() -> String {
    "/service/outlet".to_string()
}

pub async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    opts.terminal.write_line(&fmt_log!("Creating TCP Outlet"))?;
    let node = extract_address_value(&cmd.at)?;
    let project = opts
        .state
        .nodes
        .get(&node)?
        .config()
        .setup()
        .project
        .to_owned();
    let resource = Resource::new("tcp-outlet");
    if let Some(p) = project {
        if !has_policy(&node, &ctx, &opts, &resource).await? {
            add_default_project_policy(&node, &ctx, &opts, p, &resource).await?;
        }
    }

    let mut rpc = Rpc::background(&ctx, &opts, &node)?;
    let is_finished: Mutex<bool> = Mutex::new(false);

    let send_req = async {
        let new_cmd = CreateCommand {
            from: extract_address_value(&cmd.from)?,
            ..cmd
        };

        rpc.request(make_api_request(new_cmd)?).await?;

        *is_finished.lock().await = true;
        rpc.parse_response::<OutletStatus>()
    };

    let output_messages = vec![
        format!(
            "Creating outlet service on node {}...",
            &node.to_string().color(OckamColor::PrimaryResource.color()),
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
    let machine = outlet_status.worker_address()?;
    let json = serde_json::to_string_pretty(&outlet_status)?;

    opts.terminal
        .stdout()
        .plain(fmt_ok!(
            "{} is now sending TCP traffic to {}",
            &node.to_string().color(OckamColor::PrimaryResource.color()),
            &cmd.to
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ))
        .machine(machine)
        .json(json)
        .write_line()?;

    Ok(())
}

/// Construct a request to create a tcp outlet
fn make_api_request<'a>(cmd: CreateCommand) -> crate::Result<RequestBuilder<'a, CreateOutlet<'a>>> {
    let tcp_addr = cmd.to.to_string();
    let worker_addr = cmd.from;
    let alias = cmd.alias.map(|a| a.into());
    let exposed_to = cmd.exposed_to.unwrap_or(vec![]);
    let payload = CreateOutlet::new(tcp_addr, worker_addr, alias, exposed_to);
    let request = Request::post("/node/outlet").body(payload);
    Ok(request)
}
