use async_trait::async_trait;
use clap::Args;
use miette::IntoDiagnostic;
use opentelemetry::trace::FutureExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::project::ProjectsOrchestratorApi;

use crate::node_command::InMemoryNodeCommand;
use crate::shared_args::IdentityOpts;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List available Projects
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct ListCommand {
    #[command(flatten)]
    pub identity_opts: IdentityOpts,
}

#[derive(Clone)]
struct ListNodeCommand {
    opts: CommandGlobalOpts,
}

impl ListNodeCommand {
    pub fn new(opts: CommandGlobalOpts) -> Self {
        Self { opts }
    }
}

#[async_trait]
impl InMemoryNodeCommand for ListNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let is_finished: Mutex<bool> = Mutex::new(false);
        let get_projects = async {
            let projects = node.get_admin_projects().await?;
            *is_finished.lock().await = true;
            Ok(projects)
        }
        .with_current_context();

        let output_messages = vec!["Listing projects...\n".to_string()];
        let progress_output = self
            .opts
            .terminal
            .loop_messages(&output_messages, &is_finished);

        let (projects, _) = try_join!(get_projects, progress_output)?;

        let plain = self
            .opts
            .terminal
            .build_list(&projects, "No projects found")?;
        let json = serde_json::to_string(&projects).into_diagnostic()?;

        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(plain)
            .json(json)
            .write_line()?;
        Ok(())
    }
}

impl ListCommand {
    pub fn name(&self) -> String {
        "project list".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        ListNodeCommand::new(opts.clone())
            .execute(ctx, opts.state)
            .await
    }
}
