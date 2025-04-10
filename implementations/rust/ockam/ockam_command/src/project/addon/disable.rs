use async_trait::async_trait;
use clap::builder::NonEmptyStringValueParser;
use clap::Args;
use colorful::Colorful;
use std::sync::Arc;

use ockam::Context;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::addon::Addons;

use crate::node_command::InMemoryNodeCommand;
use crate::operation::util::check_for_operation_completion;
use crate::CommandGlobalOpts;

/// Disable an addon for a project
#[derive(Clone, Debug, Args)]
pub struct AddonDisableSubcommand {
    /// Project name
    #[arg(
        long = "project",
        id = "project",
        value_name = "PROJECT_NAME",
        value_parser(NonEmptyStringValueParser::new())
    )]
    project_name: String,

    /// Addon id/name
    #[arg(
        long = "addon",
        id = "addon",
        value_name = "ADDON_ID",
        value_parser(NonEmptyStringValueParser::new())
    )]
    addon_id: String,
}

#[derive(Clone)]
struct AddonDisableNodeCommand {
    opts: CommandGlobalOpts,
    command: AddonDisableSubcommand,
}

impl AddonDisableNodeCommand {
    pub fn new(opts: CommandGlobalOpts, command: AddonDisableSubcommand) -> Self {
        Self { opts, command }
    }
}

#[async_trait]
impl InMemoryNodeCommand for AddonDisableNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let project_id = self
            .opts
            .state
            .projects()
            .get_project_by_name(&self.command.project_name)
            .await?
            .project_id()
            .to_string();
        let controller = node.create_controller().await?;

        let response = controller
            .disable_addon(node.ctx(), &project_id, &self.command.addon_id)
            .await?;
        let operation_id = response.operation_id;
        check_for_operation_completion(&self.opts, &node, &operation_id, "the addon disabling")
            .await?;

        self.opts
            .terminal
            .write_line(fmt_ok!("Addon disabled successfully"))?;

        Ok(())
    }
}

impl AddonDisableSubcommand {
    pub fn name(&self) -> String {
        "project addon disable".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        AddonDisableNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}
