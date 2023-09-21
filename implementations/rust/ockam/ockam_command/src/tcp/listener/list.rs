use clap::Args;
use colorful::Colorful;
use miette::miette;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::cli_state::StateDirTrait;
use ockam_api::nodes::models::transport::TransportList;
use ockam_api::nodes::BackgroundNode;

use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::terminal::OckamColor;
use crate::util::{api, node_rpc, parse_node_name};
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List TCP listeners
#[derive(Args, Clone, Debug)]
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
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, ListCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(ctx: &Context, opts: CommandGlobalOpts, cmd: ListCommand) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node_name = parse_node_name(&node_name)?;

    if !opts.state.nodes.get(&node_name)?.is_running() {
        return Err(miette!("The node '{}' is not running", node_name));
    }

    let node = BackgroundNode::create(ctx, &opts.state, &node_name).await?;
    let is_finished: Mutex<bool> = Mutex::new(false);

    let get_transports = async {
        let transports: TransportList = node.ask(ctx, api::list_tcp_listeners()).await?;
        *is_finished.lock().await = true;
        Ok(transports)
    };

    let output_messages = vec![format!(
        "Listing TCP Listeners on {}...\n",
        node_name
            .to_string()
            .color(OckamColor::PrimaryResource.color())
    )];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (transports, _) = try_join!(get_transports, progress_output)?;

    let list = opts.terminal.build_list(
        &transports.list,
        &format!("TCP Listeners on {}", node_name),
        &format!(
            "No TCP Listeners found on {}",
            node_name
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ),
    )?;
    opts.terminal.stdout().plain(list).write_line()?;
    Ok(())
}
