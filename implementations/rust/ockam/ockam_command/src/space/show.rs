use anyhow::Context as _;
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
    #[clap(display_order = 1001)]
    pub name: String,

    #[clap(flatten)]
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
    let node_name = start_embedded_node(ctx, &opts.config).await?;
    let controller_route = cmd.cloud_opts.route();

    // Lookup space
    let id = match config::get_space(&opts.config, &cmd.name) {
        Some(id) => id,
        None => {
            config::refresh_spaces(ctx, &opts, &node_name, cmd.cloud_opts.route()).await?;
            config::get_space(&opts.config, &cmd.name)
                .context(format!("Space '{}' does not exist", cmd.name))?
        }
    };

    // Send request
    let mut rpc = RpcBuilder::new(ctx, &opts, &node_name).build();
    rpc.request(api::space::show(&id, controller_route)).await?;
    let space = rpc.parse_and_print_response::<Space>()?;
    config::set_space(&opts.config, &space)?;
    delete_embedded_node(&opts.config, rpc.node_name()).await;
    Ok(())
}
