use clap::Args;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::cloud::project::ProjectsOrchestratorApi;
use ockam_api::nodes::InMemoryNode;
use ockam_api::output::Output;

use crate::shared_args::IdentityOpts;
use crate::util::async_cmd;
use crate::{docs, output::ProjectConfigCompact, CommandGlobalOpts};

/// Show project details
#[derive(Clone, Debug, Args)]
#[command(hide = docs::hide())]
pub struct InfoCommand {
    /// Name of the project.
    #[arg(default_value = "default")]
    pub name: String,

    #[command(flatten)]
    pub identity_opts: IdentityOpts,
}

impl InfoCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "project info".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = InMemoryNode::start(ctx, &opts.state).await?;
        let project = node.get_project_by_name(ctx, &self.name).await?;
        let info = ProjectConfigCompact(project);
        opts.terminal
            .stdout()
            .plain(info.item()?)
            .json(serde_json::to_string(&info).into_diagnostic()?)
            .write_line()?;
        Ok(())
    }
}
