use clap::Args;
use colorful::Colorful;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam_api::nodes::models::portal::OutletList;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;
use ockam_node::Context;

use crate::node::NodeOpts;
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
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
) -> miette::Result<()> {
    let node = BackgroundNodeClient::create(&ctx, &opts.state, &cmd.node_opts.at_node).await?;

    let is_finished: Mutex<bool> = Mutex::new(false);

    let send_req = async {
        let res: OutletList = node.ask(&ctx, Request::get("/node/outlet")).await?;
        *is_finished.lock().await = true;
        Ok(res)
    };

    let output_messages = vec![format!(
        "Listing TCP Outlets on node {}...\n",
        node.node_name().color(OckamColor::PrimaryResource.color())
    )];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (outlets, _) = try_join!(send_req, progress_output)?;

    let list = opts.terminal.build_list(
        &outlets.list,
        &format!("Outlets on Node {}", node.node_name()),
        &format!("No TCP Outlets found on node {}.", node.node_name()),
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
