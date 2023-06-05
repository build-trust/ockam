use clap::Args;
use ockam::Context;
use ockam_api::cli_state::StateDirTrait;

use ockam_api::cloud::project::Project;

use crate::node::util::delete_embedded_node;
use crate::util::api::CloudOpts;
use crate::util::{api, node_rpc, Rpc};
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List projects
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct ListCommand {
    #[command(flatten)]
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
    let mut rpc = Rpc::embedded(ctx, &opts).await?;
    rpc.request(api::project::list(&cmd.cloud_opts.route()))
        .await?;
    let projects = rpc.parse_and_print_response::<Vec<Project>>()?;
    for project in projects {
        opts.state
            .projects
            .overwrite(&project.name, project.clone())?;
    }
    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
