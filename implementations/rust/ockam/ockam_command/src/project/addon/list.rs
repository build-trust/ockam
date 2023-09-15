use clap::builder::NonEmptyStringValueParser;
use clap::Args;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::cloud::addon::Addons;

use crate::node::util::LocalNode;
use crate::project::addon::get_project_id;
use crate::util::node_rpc;
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
    let project_id = get_project_id(&opts.state, project_name.as_str())?;

    let node = LocalNode::make(&ctx, &opts, None).await?;

    let addons = node
        .list_addons(&ctx, project_id)
        .await
        .into_diagnostic()?
        .success()
        .into_diagnostic()?;
    opts.println(&addons)?;
    Ok(())
}
