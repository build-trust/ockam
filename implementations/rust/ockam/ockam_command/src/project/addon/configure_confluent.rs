use clap::builder::NonEmptyStringValueParser;
use clap::Args;

use ockam::Context;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::cloud::addon::ConfluentConfig;
use ockam_api::cloud::operation::CreateOperationResponse;
use ockam_api::cloud::project::Project;
use ockam_api::cloud::CloudRequestWrapper;
use ockam_core::api::Request;
use ockam_core::CowStr;

use crate::node::util::delete_embedded_node;
use crate::operation::util::check_for_completion;
use crate::project::addon::configure_addon_endpoint;
use crate::project::util::check_project_readiness;
use crate::util::api::CloudOpts;

use crate::util::{api, node_rpc, Rpc};
use crate::{docs, CommandGlobalOpts, Result};

const LONG_ABOUT: &str = include_str!("./static/configure_confluent/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/configure_confluent/after_long_help.txt");

/// Configure the Confluent Cloud addon for a project
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct AddonConfigureConfluentSubcommand {
    /// Ockam project name
    #[arg(
        long = "project",
        id = "project",
        value_name = "PROJECT_NAME",
        default_value = "default",
        value_parser(NonEmptyStringValueParser::new())
    )]
    project_name: String,

    /// Confluent Cloud bootstrap server address
    #[arg(
        long,
        id = "bootstrap_server",
        value_name = "BOOTSTRAP_SERVER",
        value_parser(NonEmptyStringValueParser::new())
    )]
    bootstrap_server: String,
}

impl AddonConfigureConfluentSubcommand {
    pub fn run(self, opts: CommandGlobalOpts, cloud_opts: CloudOpts) {
        node_rpc(run_impl, (opts, cloud_opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cloud_opts, cmd): (
        CommandGlobalOpts,
        CloudOpts,
        AddonConfigureConfluentSubcommand,
    ),
) -> Result<()> {
    let controller_route = &cloud_opts.route();
    let AddonConfigureConfluentSubcommand {
        project_name,
        bootstrap_server,
    } = cmd;

    let mut rpc = Rpc::embedded(&ctx, &opts).await?;
    let body = ConfluentConfig::new(bootstrap_server);
    let addon_id = "confluent";
    let endpoint = format!(
        "{}/{}",
        configure_addon_endpoint(&opts.state, &project_name)?,
        addon_id
    );
    let req = Request::post(endpoint).body(CloudRequestWrapper::new(
        body,
        controller_route,
        None::<CowStr>,
    ));
    rpc.request(req).await?;
    let res = rpc.parse_response::<CreateOperationResponse>()?;
    let operation_id = res.operation_id;

    check_for_completion(&ctx, &opts, &cloud_opts, rpc.node_name(), &operation_id).await?;

    let project_id = opts.state.projects.get(&project_name)?.config().id.clone();
    let mut rpc = rpc.clone();
    rpc.request(api::project::show(&project_id, controller_route))
        .await?;
    let project: Project = rpc.parse_response()?;
    check_project_readiness(&ctx, &opts, &cloud_opts, rpc.node_name(), None, project).await?;

    println!("Confluent addon configured successfully");

    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
