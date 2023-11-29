use clap::Args;
use ockam_node::Context;

use crate::util::node_rpc;
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct GetDefaultNodeCommand {}

impl GetDefaultNodeCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, options);
    }
}

async fn run_impl(_ctx: Context, opts: CommandGlobalOpts) -> miette::Result<()> {
    let node_info = opts.state.get_default_node().await?;
    let addr = &node_info
        .tcp_listener_address()
        .map(|a| a.to_string())
        .unwrap_or("N/A".to_string());
    println!("Address: {addr}");
    Ok(())
}
