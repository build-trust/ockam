use clap::builder::NonEmptyStringValueParser;
use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::cloud::addon::Addons;

use crate::node::util::{delete_embedded_node, start_node_manager};
use crate::operation::util::check_for_completion;
use crate::project::addon::get_project_id;
use crate::util::node_rpc;
use crate::{fmt_ok, CommandGlobalOpts};

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
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, AddonDisableSubcommand),
) -> miette::Result<()> {
    let AddonDisableSubcommand {
        project_name,
        addon_id,
    } = cmd;
    let project_id = get_project_id(&opts.state, project_name.as_str())?;

    let node_manager = start_node_manager(&ctx, &opts, None).await?;
    let controller = node_manager
        .make_controller_client()
        .await
        .into_diagnostic()?;

    let response = controller
        .disable_addon(&ctx, project_id, addon_id)
        .await
        .into_diagnostic()?
        .success()
        .into_diagnostic()?;
    let operation_id = response.operation_id;
    check_for_completion(&opts, &ctx, &controller, &operation_id).await?;

    opts.terminal
        .write_line(&fmt_ok!("Addon disabled successfully"))?;
    delete_embedded_node(&opts, &node_manager.node_name()).await;
    Ok(())
}
