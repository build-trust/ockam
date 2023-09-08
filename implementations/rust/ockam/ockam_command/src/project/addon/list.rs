use clap::builder::NonEmptyStringValueParser;
use clap::Args;

use ockam::Context;
use ockam_api::cloud::addon::Addon;
use ockam_core::api::Request;

use crate::node::util::delete_embedded_node;
use crate::project::addon::base_endpoint;
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;

/// List available addons for a project
#[derive(Clone, Debug, Args)]
pub struct AddonListSubcommand {
    /// Project name
    #[arg(
        long = "project",
        id = "project",
        value_name = "PROJECT_NAME",
        value_parser(NonEmptyStringValueParser::new())
    )]
    project_name: String,
}

impl AddonListSubcommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, AddonListSubcommand),
) -> miette::Result<()> {
    let project_name = cmd.project_name;

    let mut rpc = Rpc::embedded(&ctx, &opts).await?;
    let req = Request::get(base_endpoint(&opts.state, &project_name)?);
    let addons: Vec<Addon> = rpc.ask(req).await?;
    opts.println(&addons)?;
    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
