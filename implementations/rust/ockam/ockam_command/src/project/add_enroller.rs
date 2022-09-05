use clap::Args;

use ockam::Context;
use ockam_api::cloud::project::Enroller;

use crate::help;
use crate::node::util::delete_embedded_node;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;

/// Adds an authorized enroller to the project' authority
#[derive(Clone, Debug, Args)]
#[clap(hide = help::hide())]
pub struct AddEnrollerCommand {
    /// Id of the project.
    #[clap(display_order = 1001)]
    pub project_id: String,

    /// Identity id to add as an authorized enroller.
    #[clap(display_order = 1002)]
    pub enroller_identity_id: String,

    /// Description of this enroller, optional.
    #[clap(display_order = 1003)]
    pub description: Option<String>,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,
}

impl AddEnrollerCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, AddEnrollerCommand),
) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: AddEnrollerCommand,
) -> crate::Result<()> {
    let mut rpc = Rpc::embedded(ctx, &opts).await?;
    rpc.request(api::project::add_enroller(&cmd)).await?;
    rpc.parse_and_print_response::<Enroller>()?;
    delete_embedded_node(&opts.config, rpc.node_name()).await;
    Ok(())
}
