use clap::builder::NonEmptyStringValueParser;
use clap::Args;

use ockam::Context;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::addon::Addons;

use crate::util::async_cmd;
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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "project addon list".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let project_name = self.project_name.clone();
        let project_id = opts
            .state
            .projects()
            .get_project_by_name(&project_name)
            .await?
            .project_id()
            .to_string();

        let node = InMemoryNode::start(ctx, &opts.state).await?;
        let controller = node.create_controller().await?;

        let addons = controller.list_addons(ctx, &project_id).await?;
        let output = opts.terminal.build_list(
            &addons,
            &format!("No addons enabled for project {project_name}"),
        )?;
        opts.terminal.stdout().plain(output).write_line()?;
        Ok(())
    }
}
