use clap::builder::NonEmptyStringValueParser;
use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_api::cloud::addon::{Addons, ConfluentConfig};
use ockam_api::nodes::InMemoryNode;

use crate::project::addon::{check_configuration_completion, get_project_id};
use crate::util::node_rpc;
use crate::{docs, fmt_ok, CommandGlobalOpts};

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
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, AddonConfigureConfluentSubcommand),
) -> miette::Result<()> {
    let AddonConfigureConfluentSubcommand {
        project_name,
        bootstrap_server,
    } = cmd;
    let project_id = get_project_id(&opts.state, project_name.as_str())?;
    let config = ConfluentConfig::new(bootstrap_server);

    let node = InMemoryNode::start(&ctx, &opts.state).await?;
    let controller = node.create_controller().await?;

    let response = controller
        .configure_confluent_addon(&ctx, project_id.clone(), config)
        .await?;
    check_configuration_completion(&opts, &ctx, &node, project_id, response.operation_id).await?;

    opts.terminal
        .write_line(&fmt_ok!("Confluent addon configured successfully"))?;

    Ok(())
}
