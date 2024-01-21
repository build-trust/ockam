use clap::Args;
use colorful::Colorful;

use ockam_api::nodes::{models, BackgroundNodeClient};
use ockam_core::api::Request;
use ockam_node::Context;

use crate::util::node_rpc;
use crate::{docs, fmt_ok, node::NodeOpts, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a Kafka Consumer
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Kafka consumer service address
    pub address: String,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    let node = BackgroundNodeClient::create(&ctx, &opts.state, &cmd.node_opts.at_node).await?;
    let req = Request::delete("/node/services/kafka_direct").body(
        models::services::DeleteServiceRequest::new(cmd.address.clone()),
    );
    node.tell(&ctx, req).await?;

    opts.terminal
        .stdout()
        .plain(fmt_ok!(
            "Kafka consumer with address `{}` successfully deleted",
            cmd.address
        ))
        .write_line()?;

    Ok(())
}
