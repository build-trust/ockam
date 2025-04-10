use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use std::sync::Arc;

use ockam::Context;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::project::ProjectsOrchestratorApi;

use crate::node_command::InMemoryNodeCommand;
use crate::shared_args::IdentityOpts;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a Project
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct DeleteCommand {
    /// Name of the space
    #[arg(display_order = 1001)]
    pub space_name: String,

    /// Name of the project
    #[arg(display_order = 1002)]
    pub project_name: String,

    #[command(flatten)]
    pub identity_opts: IdentityOpts,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

#[derive(Clone)]
struct DeleteNodeCommand {
    opts: CommandGlobalOpts,
    command: DeleteCommand,
}

impl DeleteNodeCommand {
    pub fn new(opts: CommandGlobalOpts, command: DeleteCommand) -> Self {
        Self { opts, command }
    }
}

#[async_trait]
impl InMemoryNodeCommand for DeleteNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        if self.opts.terminal.confirmed_with_flag_or_prompt(
            self.command.yes,
            "Are you sure you want to delete this project?",
        )? {
            node.delete_project_by_name(&self.command.space_name, &self.command.project_name)
                .await?;
            self.opts
                .terminal
                .clone()
                .to_stdout()
                .plain(fmt_ok!(
                    "Project with name '{}' has been deleted.",
                    &self.command.project_name
                ))
                .machine(&self.command.project_name)
                .json(serde_json::json!({ "name": &self.command.project_name }))
                .write_line()?;
        }
        Ok(())
    }
}

impl DeleteCommand {
    pub fn name(&self) -> String {
        "project delete".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        DeleteNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}
