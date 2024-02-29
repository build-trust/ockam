use clap::Args;
use miette::IntoDiagnostic;

use crate::terminal::tui::ShowCommandTui;

use ockam::Context;
use ockam_api::cloud::project::ProjectsOrchestratorApi;

use ockam_api::nodes::InMemoryNode;

use crate::output::{Output, ProjectConfigCompact};
use crate::terminal::PluralTerm;
use crate::util::api::CloudOpts;
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};
use ockam_core::AsyncTryClone;

use tracing::instrument;

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
    pub cloud_opts: CloudOpts,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "show project".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        ShowTui::run(
            ctx.async_try_clone().await.into_diagnostic()?,
            opts,
            self.name.clone(),
        )
        .await
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
    fn terminal(&self) -> crate::Terminal<crate::TerminalStream<console::Term>> {
        self.opts.terminal.clone()
    }
    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        Ok(self
            .node
            .get_admin_projects(&self.ctx)
            .await?
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
        let project = self.node.get_project_by_name(&self.ctx, item_name).await?;
        let project_output = ProjectConfigCompact(project);

        self.terminal()
            .stdout()
            .plain(project_output.output()?)
            .json(serde_json::to_string_pretty(&project_output).into_diagnostic()?)
            .write_line()?;
        Ok(())
    }
}
