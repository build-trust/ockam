use clap::Args;

use ockam::Context;
use ockam_api::nodes::models::transport::TransportStatus;
use ockam_api::nodes::BackgroundNode;
use ockam_core::api::Request;

use crate::node::NodeOpts;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show a TCP connection
#[derive(Clone, Debug, Args)]
#[command(
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ShowCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    /// TCP connection internal address or socket address
    pub address: String,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> miette::Result<()> {
    let node = BackgroundNode::create(&ctx, &opts.state, &cmd.node_opts.at_node).await?;
    let transport_status: TransportStatus = node
        .ask(
            &ctx,
            Request::get(format!("/node/tcp/connection/{}", &cmd.address)),
        )
        .await?;

    println!("TCP Connection:");
    println!("  Type: {}", transport_status.tt);
    println!("  Mode: {}", transport_status.tm);
    println!("  Socket address: {}", transport_status.socket_addr);
    println!("  Worker address: {}", transport_status.worker_addr);
    println!(
        "  Processor address: {}",
        transport_status.processor_address
    );
    println!("  Flow Control Id: {}", transport_status.flow_control_id);

    Ok(())
}
