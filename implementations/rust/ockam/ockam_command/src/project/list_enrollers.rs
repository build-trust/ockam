use clap::Args;

use ockam::Context;
use ockam_api::cloud::project::Enroller;

use crate::help;
use crate::node::util::delete_embedded_node;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;

/// List a project' authority authorized enrollers
#[derive(Clone, Debug, Args)]
#[command(hide = help::hide())]
pub struct ListEnrollersCommand {
    /// Id of the project.
    #[arg(display_order = 1001)]
    pub project_id: String,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,
}

impl ListEnrollersCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ListEnrollersCommand),
) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: ListEnrollersCommand,
) -> crate::Result<()> {
    let mut rpc = Rpc::embedded(ctx, &opts).await?;
    rpc.request(api::project::list_enrollers(&cmd)).await?;
    rpc.parse_and_print_response::<Vec<Enroller>>()?;
    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
