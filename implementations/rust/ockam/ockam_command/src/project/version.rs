use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use std::sync::Arc;

use ockam::Context;
use ockam_api::colors::color_primary;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;

use crate::node_command::InMemoryNodeCommand;
use crate::shared_args::IdentityOpts;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/version/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/version/after_long_help.txt");

/// Return the version of the Orchestrator Controller and the Projects
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::about(AFTER_LONG_HELP)
)]
pub struct VersionCommand {
    #[command(flatten)]
    pub identity_opts: IdentityOpts,
}

#[derive(Clone)]
struct VersionNodeCommand {
    opts: CommandGlobalOpts,
}

impl VersionNodeCommand {
    pub fn new(opts: CommandGlobalOpts) -> Self {
        Self { opts }
    }
}

#[async_trait]
impl InMemoryNodeCommand for VersionNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        // Send request
        let controller = node.create_controller().await?;
        let project_version = controller.get_orchestrator_version_info(node.ctx()).await?;

        let json = serde_json::to_string(&project_version).into_diagnostic()?;
        let project_version = project_version
            .project_version
            .unwrap_or("unknown".to_string());
        let plain = fmt_ok!(
            "Version of Orchestrator Controller and Projects is {}",
            color_primary(project_version.clone())
        );

        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(plain)
            .machine(project_version)
            .json(json)
            .write_line()?;
        Ok(())
    }
}

impl VersionCommand {
    pub fn name(&self) -> String {
        "project version".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        VersionNodeCommand::new(opts.clone())
            .execute(ctx, opts.state)
            .await
    }
}
