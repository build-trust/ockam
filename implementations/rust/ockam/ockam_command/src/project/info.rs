use clap::Args;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::cloud::project::ProjectsOrchestratorApi;

use ockam_api::nodes::InMemoryNode;

use crate::output::{Output, ProjectConfigCompact};
use crate::util::api::CloudOpts;
use crate::util::async_cmd;
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct InfoCommand {
    /// Name of the project.
    #[arg(default_value = "default")]
    pub name: String,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,
}

impl InfoCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "get project information".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = InMemoryNode::start(ctx, &opts.state).await?;
        let project = node.get_project_by_name(ctx, &self.name).await?;
        let info = ProjectConfigCompact(project);
        opts.terminal
            .stdout()
            .plain(info.output()?)
            .json(serde_json::to_string_pretty(&info).into_diagnostic()?)
            .write_line()?;
        Ok(())
    }
}
