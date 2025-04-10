use async_trait::async_trait;
use clap::Args;
use std::sync::Arc;

use ockam::Context;
use ockam_api::cli_state::random_name;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::project::ProjectsOrchestratorApi;

use crate::node_command::InMemoryNodeCommand;
use crate::operation::util::check_for_project_completion;
use crate::project::util::check_project_readiness;
use crate::shared_args::IdentityOpts;
use crate::util::parsers::project_name_parser;
use crate::{docs, CommandGlobalOpts};
use ockam_api::output::Output;

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a Project
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct CreateCommand {
    /// Name of the Space the project belongs to.
    #[arg(display_order = 1001)]
    pub space_name: String,

    /// Name of the project - must be unique within parent Space
    #[arg(display_order = 1002, default_value_t = random_name(), hide_default_value = true, value_parser = project_name_parser)]
    pub project_name: String,

    #[command(flatten)]
    pub identity_opts: IdentityOpts,
    //TODO:  list of admins
}

#[derive(Clone)]
struct CreateNodeCommand {
    opts: CommandGlobalOpts,
    command: CreateCommand,
}

impl CreateNodeCommand {
    pub fn new(opts: CommandGlobalOpts, command: CreateCommand) -> Self {
        Self { opts, command }
    }
}

#[async_trait]
impl InMemoryNodeCommand for CreateNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let project = node
            .create_project(&self.command.space_name, &self.command.project_name, vec![])
            .await?;
        let project = check_for_project_completion(&self.opts, &node, project).await?;
        let project = check_project_readiness(&self.opts, &node, project).await?;
        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(project.item()?)
            .json(serde_json::json!(&project))
            .write_line()?;
        Ok(())
    }
}

impl CreateCommand {
    pub fn name(&self) -> String {
        "project create".into()
    }

    pub(crate) async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        CreateNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}
