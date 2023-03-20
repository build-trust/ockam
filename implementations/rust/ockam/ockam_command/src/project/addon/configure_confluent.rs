use anyhow::Context as _;
use clap::builder::NonEmptyStringValueParser;
use clap::Args;

use ockam::Context;
use ockam_api::cloud::addon::ConfluentConfig;
use ockam_api::cloud::project::Project;
use ockam_api::cloud::CloudRequestWrapper;
use ockam_core::api::Request;
use ockam_core::CowStr;

use crate::node::util::delete_embedded_node;
use crate::project::addon::base_endpoint;
use crate::project::config;
use crate::project::util::check_project_readiness;
use crate::util::api::CloudOpts;

use crate::util::{api, node_rpc, Rpc};
use crate::{help, CommandGlobalOpts, Result};

const CONFLUENT_HELP_DETAIL: &str = r#"
About:
    Confluent Cloud addon allows you to enable end-to-end encryption with your Kafka Consumers and Kafka Producers

Examples:
    Examples of how to configure and use the Confluent Cloud addon can be found within the example documentation.
    https://docs.ockam.io/guides/examples/end-to-end-encrypted-kafka
"#;

/// Configure the Confluent Cloud addon for a project
#[derive(Clone, Debug, Args)]
#[command(after_long_help = help::template(CONFLUENT_HELP_DETAIL))]
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
        base_endpoint(&opts.config.lookup(), &project_name)?,
        addon_id
    );
    let req = Request::put(endpoint).body(CloudRequestWrapper::new(
        body,
        controller_route,
        None::<CowStr>,
    ));
    rpc.request(req).await?;
    rpc.is_ok()?;
    println!("Confluent addon enabled");

    // Wait until project is ready again
    println!("Reconfiguring project (this can take a few minutes) ...");
    tokio::time::sleep(std::time::Duration::from_secs(45)).await;

    let project_id =
        config::get_project(&opts.config, &project_name).context("project not found in lookup")?;
    let mut rpc = rpc.clone();
    rpc.request(api::project::show(&project_id, controller_route))
        .await?;
    let project: Project = rpc.parse_response()?;
    check_project_readiness(&ctx, &opts, &cloud_opts, rpc.node_name(), None, project).await?;
    println!("Confluent addon configured successfully");
    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
