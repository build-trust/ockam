use clap::Args;

use ockam::Context;

use crate::help;
use crate::node::util::delete_embedded_node;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;

/// Remove an identity as authorized enroller from the project' authority
#[derive(Clone, Debug, Args)]
#[command(hide = help::hide())]
pub struct DeleteEnrollerCommand {
    /// Id of the project.
    #[arg(display_order = 1001)]
    pub project_id: String,

    #[arg(display_order = 1002)]
    pub enroller_identity_id: String,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,
}

impl DeleteEnrollerCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteEnrollerCommand),
) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: DeleteEnrollerCommand,
) -> crate::Result<()> {
    let mut rpc = Rpc::embedded(ctx, &opts, &cmd.cloud_opts).await?;
    rpc.request(api::project::delete_enroller(&cmd)).await?;
    rpc.is_ok()?;
    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
