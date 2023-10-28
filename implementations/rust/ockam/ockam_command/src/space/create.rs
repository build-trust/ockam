use clap::Args;
use ockam::Context;
use ockam_api::cloud::space::Spaces;

use crate::output::Output;
use crate::util::api::{self, CloudOpts};
use crate::util::{is_enrolled_guard, node_rpc};
use crate::{docs, CommandGlobalOpts};
use colorful::Colorful;
use ockam_api::cli_state::random_name;
use ockam_api::cli_state::{SpaceConfig, StateDirTrait};
use ockam_api::nodes::InMemoryNode;

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a new space
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    /// Name of the space - must be unique across all Ockam Orchestrator users.
    #[arg(display_order = 1001, value_name = "SPACE_NAME", default_value_t = random_name(), hide_default_value = true, value_parser = validate_space_name)]
    pub name: String,

    /// Administrators for this space
    #[arg(display_order = 1100, last = true)]
    pub admins: Vec<String>,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &Context,
    opts: CommandGlobalOpts,
    cmd: CreateCommand,
) -> miette::Result<()> {
    is_enrolled_guard(&opts.state, None)?;

    opts.terminal.write_line(format!(
        "\n{}",
        "Creating a trial space for you (everything in it will be deleted in 15 days) ..."
            .light_magenta(),
    ))?;
    opts.terminal.write_line(format!(
        "{}",
        "To learn more about production ready spaces in Ockam Orchestrator, contact us at: hello@ockam.io".light_magenta()
    ))?;

    let node = InMemoryNode::start(ctx, &opts.state).await?;
    let controller = node.create_controller().await?;
    let space = controller.create_space(ctx, cmd.name, cmd.admins).await?;

    opts.terminal
        .stdout()
        .plain(space.output()?)
        .json(serde_json::json!(&space))
        .write_line()?;
    opts.state
        .spaces
        .overwrite(&space.name, SpaceConfig::from(&space))?;
    Ok(())
}

fn validate_space_name(s: &str) -> Result<String, String> {
    match api::validate_cloud_resource_name(s) {
        Ok(_) => Ok(s.to_string()),
        Err(_e)=> Err(String::from(
            "space name can contain only alphanumeric characters and the '-', '_' and '.' separators. \
            Separators must occur between alphanumeric characters. This implies that separators can't \
            occur at the start or end of the name, nor they can occur in sequence.",
        ))
    }
}
