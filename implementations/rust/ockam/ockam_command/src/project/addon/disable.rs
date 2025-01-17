use clap::builder::NonEmptyStringValueParser;
use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::addon::Addons;

use crate::operation::util::check_for_operation_completion;
use crate::util::async_cmd;
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

impl AddonDisableSubcommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "project addon disable".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let project_id = opts
            .state
            .projects()
            .get_project_by_name(&self.project_name)
            .await?
            .project_id()
            .to_string();
        let node = InMemoryNode::start(ctx, &opts.state).await?;
        let controller = node.create_controller().await?;

        let response = controller
            .disable_addon(ctx, &project_id, &self.addon_id)
            .await?;
        let operation_id = response.operation_id;
        check_for_operation_completion(&opts, ctx, &node, &operation_id, "the addon disabling")
            .await?;

        opts.terminal
            .write_line(fmt_ok!("Addon disabled successfully"))?;
        Ok(())
    }
}
