use clap::Args;

use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::util::extract_address_value;
use ockam::Context;
use ockam_api::nodes::models;
use ockam_core::api::Request;

use crate::util::{node_rpc, Rpc};
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

    /// TCP connection Worker Address or Tcp Address
    pub address: String,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node = extract_address_value(&node_name)?;
    let mut rpc = Rpc::background(&ctx, &opts, &node)?;
    rpc.request(Request::get(format!(
        "/node/tcp/connection/{}",
        &cmd.address
    )))
    .await?;
    let res = rpc.parse_response_body::<models::transport::TransportStatus>()?;

    println!("TCP Connection:");
    println!("  Type: {}", res.tt);
    println!("  Mode: {}", res.tm);
    println!("  Socket address: {}", res.socket_addr);
    println!("  Worker address: {}", res.worker_addr);
    println!("  Processor address: {}", res.processor_address);
    println!("  Flow Control Id: {}", res.flow_control_id);

    Ok(())
}
