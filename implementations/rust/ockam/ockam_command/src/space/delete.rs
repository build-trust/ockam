use clap::Args;

use ockam::Context;

use crate::node::create::start_embedded_node;
use crate::space::util::config;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, RpcBuilder};
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    /// Name of the space.
    #[clap(display_order = 1001)]
    pub name: String,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
) -> crate::Result<()> {
    let node_name = start_embedded_node(ctx, &opts.config).await?;
    let controller_route = cmd.cloud_opts.route();

    // Try to remove from config, in case the space was removed from the cloud but not from the config file.
    let _ = config::remove_space(&opts.config, &cmd.name);

    // Lookup space
    let id = match config::get_space(&opts.config, &cmd.name) {
        Some(id) => id,
        None => {
            // The space is not in the config file.
            // Fetch all available spaces from the cloud.
            config::refresh_spaces(ctx, &opts, &node_name, controller_route).await?;

            // If the space is not found in the lookup, then it must not exist in the cloud, so we exit the command.
            match config::get_space(&opts.config, &cmd.name) {
                Some(id) => id,
                None => {
                    return Ok(());
                }
            }
        }
    };

    // Send request
    let mut rpc = RpcBuilder::new(ctx, &opts, &node_name).build();
    rpc.request(api::space::delete(&id, controller_route))
        .await?;
    rpc.is_ok()?;

    // Try to remove from config again, in case it was re-added after the refresh.
    let _ = config::remove_space(&opts.config, &cmd.name);

    Ok(())
}
