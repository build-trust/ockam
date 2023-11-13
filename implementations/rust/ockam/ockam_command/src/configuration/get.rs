use clap::Args;

use ockam_node::Context;

use crate::util::node_rpc;
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct GetCommand {
    /// Alias name of the node
    pub alias: String,
}

impl GetCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, GetCommand),
) -> miette::Result<()> {
    let node_info = opts.state.get_node(&cmd.alias).await?;
    let addr = &node_info
        .tcp_listener_address()
        .map(|a| a.to_string())
        .unwrap_or("N/A".to_string());
    println!("Address: {addr}");
    Ok(())
}
