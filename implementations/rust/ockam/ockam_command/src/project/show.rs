use async_trait::async_trait;
use clap::Args;
use miette::IntoDiagnostic;
use tracing::instrument;

use ockam::Context;
use ockam_api::cloud::project::ProjectsOrchestratorApi;
use ockam_api::nodes::InMemoryNode;
use ockam_api::output::Output;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_core::TryClone;

use crate::shared_args::{IdentityOpts, RetryOpts};
use crate::terminal::tui::ShowCommandTui;
use crate::tui::PluralTerm;
use crate::{docs, Command, CommandGlobalOpts, Error};

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

    fn retry_opts(&self) -> Option<RetryOpts> {
        Some(self.retry_opts.clone())
    }

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        Ok(ShowTui::run(ctx.try_clone().into_diagnostic()?, opts, self.name.clone()).await?)
    }
}

pub struct ShowTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    project_name: Option<String>,
    node: InMemoryNode,
}

impl ShowTui {
    pub async fn run(
        ctx: Context,
        opts: CommandGlobalOpts,
        project_name: Option<String>,
    ) -> miette::Result<()> {
        let node = InMemoryNode::start(&ctx, &opts.state).await?;
        let tui = Self {
            ctx,
            opts,
            project_name,
            node,
        };
        tui.show().await
    }
}

#[ockam_core::async_trait]
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
            .get_admin_projects(&self.ctx)
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

    #[instrument(skip_all)]
    async fn show_single(&self, item_name: &str) -> miette::Result<()> {
        let project = self
            .node
            .get_project_by_name(&self.ctx, item_name)
            .await
            .map_err(Error::Retry)?;

        self.terminal()
            .stdout()
            .plain(project.item()?)
            .json_obj(project)?
            .write_line()?;
        Ok(())
    }
}
