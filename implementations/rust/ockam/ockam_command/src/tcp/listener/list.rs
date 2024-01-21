use clap::Args;
use colorful::Colorful;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::nodes::models::transport::TransportList;
use ockam_api::nodes::BackgroundNodeClient;

use crate::node::NodeOpts;
use crate::terminal::OckamColor;
use crate::util::{api, node_rpc};
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
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, ListCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(ctx: &Context, opts: CommandGlobalOpts, cmd: ListCommand) -> miette::Result<()> {
    let node = BackgroundNodeClient::create(ctx, &opts.state, &cmd.node_opts.at_node).await?;
    let is_finished: Mutex<bool> = Mutex::new(false);

    let get_transports = async {
        let transports: TransportList = node.ask(ctx, api::list_tcp_listeners()).await?;
        *is_finished.lock().await = true;
        Ok(transports)
    };

    let output_messages = vec![format!(
        "Listing TCP Listeners on {}...\n",
        node.node_name().color(OckamColor::PrimaryResource.color())
    )];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (transports, _) = try_join!(get_transports, progress_output)?;

    let list = opts.terminal.build_list(
        &transports.list,
        &format!("TCP Listeners on {}", node.node_name()),
        &format!(
            "No TCP Listeners found on {}",
            node.node_name().color(OckamColor::PrimaryResource.color())
        ),
    )?;
    opts.terminal.stdout().plain(list).write_line()?;
    Ok(())
}
