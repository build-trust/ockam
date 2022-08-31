use anyhow::Context as _;
use clap::Args;

use ockam::{Context, TcpTransport};

use crate::node::NodeOpts;
use crate::project::util::config;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, RpcBuilder};
use crate::{space, CommandGlobalOpts};

/// Delete projects
#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    /// Name of the space.
    #[clap(display_order = 1001)]
    pub space_name: String,

    /// Name of the project.
    #[clap(display_order = 1002)]
    pub project_name: String,

    #[clap(flatten)]
    pub node_opts: NodeOpts,

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
    let space_id = space::config::get_space(&opts.config, &cmd.space_name)
        .context(format!("Space '{}' does not exist", cmd.space_name))?;

    let tcp = TcpTransport::create(ctx).await?;
    let controller_route = cmd.cloud_opts.route();

    // Try to remove from config, in case the project was removed from the cloud but not from the config file.
    let _ = config::remove_project(&opts.config, &cmd.project_name);

    // Lookup project
    let project_id = match config::get_project(&opts.config, &cmd.project_name) {
        Some(id) => id,
        None => {
            // The project is not in the config file.
            // Fetch all available projects from the cloud.
            config::refresh_projects(ctx, &opts, &tcp, &cmd.node_opts.api_node, controller_route)
                .await?;

            // If the project is not found in the lookup, then it must not exist in the cloud, so we exit the command.
            match config::get_project(&opts.config, &cmd.project_name) {
                Some(id) => id,
                None => {
                    return Ok(());
                }
            }
        }
    };

    // Send request
    let mut rpc = RpcBuilder::new(ctx, &opts, &cmd.node_opts.api_node)
        .tcp(&tcp)?
        .build();
    rpc.request(api::project::delete(
        &space_id,
        &project_id,
        controller_route,
    ))
    .await?;
    rpc.is_ok()?;

    // Try to remove from config again, in case it was re-added after the refresh.
    let _ = config::remove_project(&opts.config, &cmd.project_name);

    Ok(())
}
