use clap::builder::NonEmptyStringValueParser;
use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_api::cloud::addon::DisableAddon;
use ockam_api::cloud::operation::CreateOperationResponse;
use ockam_api::cloud::CloudRequestWrapper;
use ockam_core::api::Request;

use crate::node::util::delete_embedded_node;
use crate::operation::util::check_for_completion;
use crate::project::addon::disable_addon_endpoint;
use crate::util::{node_rpc, Rpc};
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

    let mut rpc = Rpc::embedded(&ctx, &opts).await?;
    let body = DisableAddon::new(addon_id);
    let endpoint = disable_addon_endpoint(&opts.state, &project_name)?;

    let req = Request::post(endpoint).body(CloudRequestWrapper::new(body));
    let response: CreateOperationResponse = rpc.ask(req).await?;
    let operation_id = response.operation_id;

    check_for_completion(&opts, &rpc, &operation_id).await?;
    opts.terminal
        .write_line(&fmt_ok!("Addon disabled successfully"))?;
    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
