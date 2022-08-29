use clap::Args;

use ockam::Context;
use ockam_api::cloud::space::Space;

use crate::node::NodeOpts;
use crate::space::util::config;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;

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
    let mut rpc = Rpc::new(ctx, &opts, &cmd.node_opts.api_node)?;
    rpc.request(api::space::list(cmd.cloud_opts.route()))
        .await?;
    let spaces = rpc.parse_and_print_response::<Vec<Space>>()?;
    config::set_spaces(&opts.config, &spaces)?;
    Ok(())
}
