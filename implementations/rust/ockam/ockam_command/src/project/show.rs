use async_trait::async_trait;
use clap::Args;
use std::sync::Arc;
use tracing::{instrument, Level};

use crate::node_command::InMemoryNodeCommand;
use crate::shared_args::{IdentityOpts, RetryOpts};
use crate::terminal::tui::ShowCommandTui;
use crate::tui::PluralTerm;
use crate::{docs, Command, CommandGlobalOpts, Error};
use ockam::Context;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::project::ProjectsOrchestratorApi;
use ockam_api::output::Output;
use ockam_api::terminal::{Terminal, TerminalStream};

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show detailed Project information
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct ShowCommand {
    /// Name of the project.
    #[arg(display_order = 1001)]
    pub name: Option<String>,

    #[command(flatten)]
    pub identity_opts: IdentityOpts,

    #[command(flatten)]
    pub retry_opts: RetryOpts,
}

#[async_trait]
impl Command for ShowCommand {
    const NAME: &'static str = "project show";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        ShowTuiNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}

#[derive(Clone)]
pub struct ShowTuiNodeCommand {
    opts: CommandGlobalOpts,
    command: ShowCommand,
}

impl ShowTuiNodeCommand {
    pub fn new(opts: CommandGlobalOpts, command: ShowCommand) -> Self {
        Self { opts, command }
    }
}

#[async_trait]
impl InMemoryNodeCommand for ShowTuiNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let tui = ShowTui {
            opts: self.opts.clone(),
            project_name: self.command.name.clone(),
            node,
        };
        tui.show().await
    }
}

struct ShowTui {
    opts: CommandGlobalOpts,
    project_name: Option<String>,
    node: Arc<InMemoryNode>,
}

#[async_trait]
impl ShowCommandTui for ShowTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Project;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.project_name.clone()
    }
    fn terminal(&self) -> Terminal<TerminalStream<console::Term>> {
        self.opts.terminal.clone()
    }
    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        Ok(self
            .node
            .get_admin_projects()
            .await
            .map_err(Error::Retry)?
            .iter()
            .map(|p| p.name().to_string())
            .collect())
    }

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String> {
        let project = match self.cmd_arg_item_name() {
            Some(command) => command.to_owned(),
            None => self
                .opts
                .state
                .projects()
                .get_default_project()
                .await?
                .name()
                .to_string(),
        };
        Ok(project)
    }

    #[instrument(skip_all, level = Level::TRACE)]
    async fn show_single(&self, item_name: &str) -> miette::Result<()> {
        let project = self
            .node
            .get_project_by_name(item_name)
            .await
            .map_err(Error::Retry)?;

        self.terminal()
            .to_stdout()
            .plain(project.item()?)
            .json_obj(project)?
            .write_line()?;
        Ok(())
    }
}
