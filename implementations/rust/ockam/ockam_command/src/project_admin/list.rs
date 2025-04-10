use crate::node_command::InMemoryNodeCommand;
use crate::shared_args::IdentityOpts;
use crate::{Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use ockam::Context;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::project::ProjectsOrchestratorApi;
use std::sync::Arc;

/// List the Admins of a Project
#[derive(Clone, Debug, Args)]
#[command()]
pub struct ListCommand {
    /// Name of the Project
    name: Option<String>,

    #[command(flatten)]
    identity_opts: IdentityOpts,
}

#[derive(Clone)]
struct ListNodeCommand {
    opts: CommandGlobalOpts,
    command: ListCommand,
}
impl ListNodeCommand {
    pub fn new(opts: CommandGlobalOpts, command: ListCommand) -> Self {
        Self { opts, command }
    }
}
#[async_trait]
impl InMemoryNodeCommand for ListNodeCommand {
    fn project_name(&self) -> Option<String> {
        self.command.name.clone()
    }

    fn identity_name(&self) -> Option<String> {
        self.command.identity_opts.identity_name.clone()
    }

    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let project = self.get_project(node.clone()).await?;
        let admins = node.list_project_admins(project.project_id()).await?;

        let list = &self.opts.terminal.build_list(&admins, "No admins found")?;
        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(list)
            .json_obj(admins)?
            .write_line()?;
        Ok(())
    }
}

#[async_trait]
impl Command for ListCommand {
    const NAME: &'static str = "project-admin list";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        ListNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}
