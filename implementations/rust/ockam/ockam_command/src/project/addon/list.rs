use clap::Args;
use clap::builder::NonEmptyStringValueParser;

use ockam::Context;
use ockam_api::cloud::addon::Addon;
use ockam_api::cloud::CloudRequestWrapper;
use ockam_core::api::Request;

use crate::CommandGlobalOpts;
use crate::node::util::delete_embedded_node;
use crate::project::addon::base_endpoint;
use crate::util::{node_rpc, Rpc};
use crate::util::api::CloudOpts;

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
    pub fn run(self, opts: CommandGlobalOpts, cloud_opts: CloudOpts) {
        node_rpc(run_impl, (opts, cloud_opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cloud_opts, cmd): (CommandGlobalOpts, CloudOpts, AddonListSubcommand),
) -> miette::Result<()> {
    let controller_route = &cloud_opts.route();
    let project_name = cmd.project_name;

    let mut rpc = Rpc::embedded(&ctx, &opts).await?;
    let req = Request::get(base_endpoint(&opts.state, &project_name)?)
        .body(CloudRequestWrapper::bare(controller_route));
    rpc.request(req).await?;
    rpc.parse_and_print_response::<Vec<Addon>>()?;
    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
