use async_trait::async_trait;
use clap::builder::NonEmptyStringValueParser;
use clap::Args;
use std::sync::Arc;

use ockam::Context;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::addon::Addons;

use crate::node_command::InMemoryNodeCommand;
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

#[derive(Clone)]
struct AddonListNodeCommand {
    opts: CommandGlobalOpts,
    command: AddonListSubcommand,
}

#[async_trait]
impl InMemoryNodeCommand for AddonListNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let project_name = self.command.project_name.clone();
        let project_id = self
            .opts
            .state
            .projects()
            .get_project_by_name(&project_name)
            .await?
            .project_id()
            .to_string();

        let controller = node.create_controller().await?;

        let addons = controller.list_addons(node.ctx(), &project_id).await?;
        let output = self.opts.terminal.build_list(
            &addons,
            &format!("No addons enabled for project {project_name}"),
        )?;
        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(output)
            .write_line()?;
        Ok(())
    }
}

impl AddonListNodeCommand {
    pub fn new(opts: CommandGlobalOpts, command: AddonListSubcommand) -> Self {
        Self { opts, command }
    }
}

impl AddonListSubcommand {
    pub fn name(&self) -> String {
        "project addon list".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        AddonListNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}
