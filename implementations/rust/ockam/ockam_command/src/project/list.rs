use clap::Args;
use ockam::Context;

use ockam_api::cloud::project::Project;

use crate::node::create::start_embedded_node;
use crate::node::NodeOpts;
use crate::project::util::config;
use crate::util::api::CloudOpts;
use crate::util::{api, node_rpc, RpcBuilder};
use crate::CommandGlobalOpts;

/// List projects
#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    #[clap(flatten)]
    pub node_opts: NodeOpts,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,
}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
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
    let node_name = start_embedded_node(ctx, &opts.config).await?;
    let mut rpc = RpcBuilder::new(ctx, &opts, &node_name).build();
    rpc.request(api::project::list(cmd.cloud_opts.route()))
        .await?;
    let projects = rpc.parse_and_print_response::<Vec<Project>>()?;
    config::set_projects(&opts.config, &projects)?;
    Ok(())
}
