use clap::Args;
use miette::IntoDiagnostic;
use ockam_api::cloud::Controller;

use crate::terminal::tui::ShowCommandTui;

use ockam::Context;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::cloud::project::{Project, Projects};

use tokio::try_join;

use ockam_api::nodes::InMemoryNode;

use crate::output::Output;
use crate::util::api::CloudOpts;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};
use tokio::sync::Mutex;

use super::util::refresh_projects;

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
    controller: Controller,
}
impl ProjectShowTui {
    pub async fn run(
        ctx: Context,
        opts: CommandGlobalOpts,
        project_name: Option<String>,
    ) -> miette::Result<()> {
        let node = InMemoryNode::start(&ctx, &opts.state).await?;
        let controller = node.create_controller().await?;
        //Refrsh the project list
        refresh_projects(&opts, &ctx, &controller).await?;
        let tui = Self {
            ctx,
            opts,
            project_name,
            controller,
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
        Ok(self.opts.state.projects.list_items_names()?)
    }

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String> {
        let project = match self.cmd_arg_item_name() {
            Some(command) => command.to_owned(),
            None => self.opts.state.projects.default()?.name().to_owned(),
        };
        Ok(project)
    }
    async fn show_single(&self, item_name: &str) -> miette::Result<()> {
        let project_state = &self.opts.state.projects.get(item_name)?;
        let id = project_state.config().id.clone();
        let project = self.controller.get_project(&self.ctx, id).await?;

        self.terminal()
            .stdout()
            .plain(project.output()?)
            .json(serde_json::to_string_pretty(&project).into_diagnostic()?)
            .write_line()?;

        self.opts
            .state
            .projects
            .overwrite(&project.name, project.clone())?;
        Ok(())
    }
    async fn show_multiple(&self, selected_items_names: Vec<String>) -> miette::Result<()> {
        let is_finished: Mutex<bool> = Mutex::new(false);
        let terminal = self.terminal();
        let mut projects_list: Vec<Project> = Vec::with_capacity(selected_items_names.len());
        let get_projects = async {
            for project_name in selected_items_names.iter() {
                let project_state = &self.opts.state.projects.get(project_name)?;
                let id = project_state.config().id.clone();
                let project = self.controller.get_project(&self.ctx, id).await?;
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
        let json = serde_json::to_string_pretty(&projects).into_diagnostic()?;

        for project in projects {
            self.opts
                .state
                .projects
                .overwrite(&project.name, project.clone())?;
        }

        self.terminal()
            .stdout()
            .plain(plain)
            .json(json)
            .write_line()?;
        Ok(())
    }
}
