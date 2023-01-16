use clap::Args;

use ockam::Context;
use ockam_api::cloud::space::Space;

use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::space::util::config;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, RpcBuilder};
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    /// Name of the space.
    #[arg(display_order = 1001)]
    pub name: String,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, ShowCommand)) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: ShowCommand,
) -> crate::Result<()> {
    let node_name = start_embedded_node(ctx, &opts).await?;
    let controller_route = &cmd.cloud_opts.route();

    // Lookup space
    let id = config::get_space(
        ctx,
        &opts,
        &cmd.name,
        &node_name,
        &cmd.cloud_opts.route(),
        None,
    )
    .await?;

    // Send request
    let mut rpc = RpcBuilder::new(ctx, &opts, &node_name).build();
    rpc.request(api::space::show(&id, controller_route)).await?;
    let space = rpc.parse_and_print_response::<Space>()?;
    config::set_space(&opts.state.nodes.get(&node_name)?, &space)?;
    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
