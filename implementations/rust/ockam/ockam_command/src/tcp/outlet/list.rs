use clap::Args;
use ockam::Context;
use ockam_api::nodes::models;

use crate::{
    node::NodeOpts,
    util::{api, node_rpc, Rpc},
    CommandGlobalOpts,
};

#[derive(Args, Clone, Debug)]
pub struct ListCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,
}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(tcp_outlet_list_rcp, (options, self))
    }
}

async fn tcp_outlet_list_rcp(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
) -> crate::Result<()> {
    tcp_outlet_list_rcp_impl(&mut ctx, opts, cmd).await
}

async fn tcp_outlet_list_rcp_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: ListCommand,
) -> crate::Result<()> {
    let mut rpc = Rpc::background(ctx, &opts, &cmd.node_opts.api_node)?;
    rpc.request(api::tcp::outlet::list()).await?;
    let res = rpc.parse_response::<models::portal::InletList>()?;

    println!("Tcp Outlets for node `{}`:", &cmd.node_opts.api_node);

    for outlet in res.list {
        println!(
            "Alias: {:?}\nAddress: {:?}\nWorker Address: {:?}",
            outlet.alias, outlet.bind_addr, outlet.worker_addr
        );
    }

    Ok(())
}
