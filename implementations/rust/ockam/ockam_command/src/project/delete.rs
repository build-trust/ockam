use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::project::ProjectsOrchestratorApi;

use crate::shared_args::IdentityOpts;
use crate::util::async_cmd;
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

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "project delete".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        if opts.terminal.confirmed_with_flag_or_prompt(
            self.yes,
            "Are you sure you want to delete this project?",
        )? {
            let node = InMemoryNode::start(ctx, &opts.state).await?;

            node.delete_project_by_name(ctx, &self.space_name, &self.project_name)
                .await?;
            opts.terminal
                .stdout()
                .plain(fmt_ok!(
                    "Project with name '{}' has been deleted.",
                    &self.project_name
                ))
                .machine(&self.project_name)
                .json(serde_json::json!({ "name": &self.project_name }))
                .write_line()?;
        }
        Ok(())
    }
}
