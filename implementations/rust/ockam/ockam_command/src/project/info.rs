use async_trait::async_trait;
use clap::Args;
use miette::IntoDiagnostic;
use std::sync::Arc;

use ockam::Context;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::project::ProjectsOrchestratorApi;
use ockam_api::output::Output;

use crate::node_command::InMemoryNodeCommand;
use crate::shared_args::IdentityOpts;
use crate::{docs, CommandGlobalOpts};

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

#[derive(Clone)]
struct InfoNodeCommand {
    opts: CommandGlobalOpts,
    command: InfoCommand,
}

impl InfoNodeCommand {
    pub fn new(opts: CommandGlobalOpts, command: InfoCommand) -> Self {
        Self { opts, command }
    }
}

#[async_trait]
impl InMemoryNodeCommand for InfoNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let project = node.get_project_by_name(&self.command.name).await?;
        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(project.item()?)
            .json(serde_json::to_string(&project).into_diagnostic()?)
            .write_line()?;
        Ok(())
    }
}

impl InfoCommand {
    pub fn name(&self) -> String {
        "project information".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        InfoNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}
