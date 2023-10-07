use clap::Args;
use colorful::Colorful;
use miette::miette;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam_api::address::extract_address_value;
use ockam_api::cli_state::StateDirTrait;
use ockam_api::nodes::models::portal::OutletList;
use ockam_api::nodes::BackgroundNode;
use ockam_core::api::Request;
use ockam_node::Context;

use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::terminal::OckamColor;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List TCP Outlets on the default node
#[derive(Clone, Debug, Args)]
#[command(
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ListCommand {
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node_name = extract_address_value(&node_name)?;

    if !opts.state.nodes.get(&node_name)?.is_running() {
        return Err(miette!("The node '{}' is not running", node_name));
    }

    let is_finished: Mutex<bool> = Mutex::new(false);

    let send_req = async {
        let res = send_request(&ctx, &opts, node_name.clone()).await;
        *is_finished.lock().await = true;
        res
    };

    let output_messages = vec![format!(
        "Listing TCP Outlets on node {}...\n",
        node_name
            .to_string()
            .color(OckamColor::PrimaryResource.color())
    )];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (outlets, _) = try_join!(send_req, progress_output)?;

    let list = opts.terminal.build_list(
        &outlets.list,
        &format!("Outlets on Node {node_name}"),
        &format!("No TCP Outlets found on node {node_name}."),
    )?;
    let json: Vec<_> = outlets
        .list
        .iter()
        .map(|outlet| {
            Ok(serde_json::json!({
                "alias": outlet.alias,
                "from": outlet.worker_address()?,
                "to": outlet.socket_addr,
            }))
        })
        .flat_map(|res: Result<_, ockam_core::Error>| res.ok())
        .collect();
    opts.terminal
        .stdout()
        .plain(list)
        .json(serde_json::json!(json))
        .write_line()?;

    Ok(())
}

pub async fn send_request(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    to_node: impl Into<Option<String>>,
) -> crate::Result<OutletList> {
    let node_name = get_node_name(&opts.state, &to_node.into());
    let node = BackgroundNode::create(ctx, &opts.state, &node_name).await?;
    Ok(node.ask(ctx, Request::get("/node/outlet")).await?)
}
