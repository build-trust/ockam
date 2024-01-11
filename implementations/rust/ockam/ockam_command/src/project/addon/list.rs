use clap::builder::NonEmptyStringValueParser;
use clap::Args;

use ockam::Context;
use ockam_api::cloud::addon::Addons;
use ockam_api::nodes::InMemoryNode;

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
        node_rpc(opts.rt.clone(), run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, AddonListSubcommand),
) -> miette::Result<()> {
    let project_name = cmd.project_name;
    let project_id = &opts.state.get_project_by_name(&project_name).await?.id();

    let node = InMemoryNode::start(&ctx, &opts.state).await?;
    let controller = node.create_controller().await?;

    let addons = controller.list_addons(&ctx, project_id).await?;
    let output = opts.terminal.build_list(
        &addons,
        &format!("Addons for project {project_name}"),
        &format!("No addons enabled for project {project_name}"),
    )?;
    opts.terminal.stdout().plain(output).write_line()?;
    Ok(())
}
