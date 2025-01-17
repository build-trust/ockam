use crate::shared_args::IdentityOpts;
use crate::{Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use ockam::Context;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::project::ProjectsOrchestratorApi;

/// List the Admins of a Project
#[derive(Clone, Debug, Args)]
#[command()]
pub struct ListCommand {
    /// Name of the Project
    name: Option<String>,

    #[command(flatten)]
    identity_opts: IdentityOpts,
}

#[async_trait]
impl Command for ListCommand {
    const NAME: &'static str = "project-admin list";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        let project = opts
            .state
            .projects()
            .get_project_by_name_or_default(&self.name)
            .await?;
        let node = InMemoryNode::start_with_identity_and_project_name(
            ctx,
            &opts.state,
            self.identity_opts.identity_name,
            Some(project.project_name().to_string()),
        )
        .await?;
        let admins = node.list_project_admins(ctx, project.project_id()).await?;

        let list = &opts.terminal.build_list(&admins, "No admins found")?;
        opts.terminal
            .stdout()
            .plain(list)
            .json_obj(admins)?
            .write_line()?;
        Ok(())
    }
}
