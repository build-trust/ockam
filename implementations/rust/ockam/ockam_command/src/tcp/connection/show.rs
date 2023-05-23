use clap::Args;

use crate::node::NodeOpts;
use crate::util::extract_address_value;
use ockam::Context;
use ockam_api::nodes::models;
use ockam_core::api::Request;

use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;

/// Show a TCP connection
#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    /// TCP connection ID
    pub id: String,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> crate::Result<()> {
    let node = extract_address_value(&cmd.node_opts.api_node)?;
    let mut rpc = Rpc::background(&ctx, &opts, &node)?;
    rpc.request(Request::get(format!("/node/tcp/connection/{}", &cmd.id)))
        .await?;
    let res = rpc.parse_response::<models::transport::TransportStatus>()?;

    println!("TCP Connection:");
    println!("  ID: {}", res.tid);
    println!("  Type: {}", res.tt);
    println!("  Mode: {}", res.tm);
    println!("  Socket address: {}", res.socket_addr);
    println!("  Worker address: {}", res.worker_addr);

    Ok(())
}
