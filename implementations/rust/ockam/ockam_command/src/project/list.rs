use clap::Args;
use ockam::Context;

use ockam_api::cloud::project::Project;

use crate::node::NodeOpts;
use crate::project::util::config;
use crate::util::api::CloudOpts;
use crate::util::{api, node_rpc, Rpc};
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    #[clap(flatten)]
    pub node_opts: NodeOpts,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,
}

impl ListCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: ListCommand) {
        node_rpc(rpc, (opts, cmd));
    }
}

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, ListCommand)) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: ListCommand,
) -> crate::Result<()> {
    let mut rpc = Rpc::new(ctx, &opts, &cmd.node_opts.api_node)?;
    rpc.request(api::project::list(cmd.cloud_opts.route()))
        .await?;
    let projects = rpc.parse_and_print_response::<Vec<Project>>()?;
    config::set_projects(&opts.config, &projects)?;
    Ok(())
}
