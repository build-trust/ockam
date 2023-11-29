use clap::Args;
use miette::IntoDiagnostic;

use crate::terminal::tui::ShowCommandTui;

use ockam::Context;
use ockam_api::cloud::project::{Project, Projects};

use tokio::try_join;

use ockam_api::nodes::InMemoryNode;

use crate::output::{Output, ProjectConfigCompact};
use crate::util::api::CloudOpts;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};
use tokio::sync::Mutex;

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show projects
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
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, ShowCommand)) -> miette::Result<()> {
    run_impl(ctx, opts, cmd).await
}

async fn run_impl(ctx: Context, opts: CommandGlobalOpts, cmd: ShowCommand) -> miette::Result<()> {
    ProjectShowTui::run(ctx, opts, cmd.name).await
}

pub struct ProjectShowTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    project_name: Option<String>,
    node: InMemoryNode,
}
impl ProjectShowTui {
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
impl ShowCommandTui for ProjectShowTui {
    const ITEM_NAME: &'static str = "project";

    fn cmd_arg_item_name(&self) -> Option<&str> {
        self.project_name.as_deref()
    }
    fn terminal(&self) -> crate::Terminal<crate::TerminalStream<console::Term>> {
        self.opts.terminal.clone()
    }
    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        Ok(self
            .opts
            .state
            .get_projects()
            .await?
            .iter()
            .map(|p| p.name())
            .collect())
    }

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String> {
        let project = match self.cmd_arg_item_name() {
            Some(command) => command.to_owned(),
            None => self.opts.state.get_default_project().await?.name(),
        };
        Ok(project)
    }
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
    async fn show_multiple(&self, selected_items_names: Vec<String>) -> miette::Result<()> {
        let is_finished: Mutex<bool> = Mutex::new(false);
        let terminal = self.terminal();
        let mut projects_list: Vec<Project> = Vec::with_capacity(selected_items_names.len());
        let get_projects = async {
            for project_name in selected_items_names.iter() {
                let project = self
                    .node
                    .get_project_by_name(&self.ctx, project_name)
                    .await?;
                projects_list.push(project)
            }
            *is_finished.lock().await = true;
            Ok(projects_list)
        };

        let output_messages = vec![format!("Listing projects...\n",)];
        let progress_output = terminal.progress_output(&output_messages, &is_finished);

        let (projects, _) = try_join!(get_projects, progress_output)?;

        let plain = self.terminal().build_list(
            &projects,
            "Projects",
            "No projects found on this system.",
        )?;
        let mut project_outputs = vec![];
        for project in projects {
            project_outputs.push(ProjectConfigCompact(project));
        }
        let json = serde_json::to_string_pretty(&project_outputs).into_diagnostic()?;

        self.terminal()
            .stdout()
            .plain(plain)
            .json(json)
            .write_line()?;
        Ok(())
    }
}
