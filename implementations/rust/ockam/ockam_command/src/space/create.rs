use clap::Args;
use ockam::Context;
use ockam_api::cloud::space::Space;
use rand::prelude::random;

use crate::node::util::delete_embedded_node;
use crate::util::api::{self};
use crate::util::{node_rpc, Rpc};
use crate::{docs, CommandGlobalOpts};
use colorful::Colorful;
use ockam_api::cli_state::{SpaceConfig, StateDirTrait};

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
    #[arg(display_order = 1001, value_name = "SPACE_NAME", default_value_t = hex::encode(&random::<[u8;4]>()), hide_default_value = true, value_parser = validate_space_name)]
    pub name: String,

    /// Administrators for this space
    #[arg(display_order = 1100, last = true)]
    pub admins: Vec<String>,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        println!(
            "\n{}",
            "Creating a trial space for you (everything in it will be deleted in 15 days) ..."
                .light_magenta()
        );
        println!(
            "{}",
            "To learn more about production ready spaces in Ockam Orchestrator, contact us at: hello@ockam.io".light_magenta()
        );
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> miette::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: CreateCommand,
) -> miette::Result<()> {
    let mut rpc = Rpc::embedded(ctx, &opts).await?;
    rpc.request(api::space::create(cmd)).await?;
    let space = rpc.parse_and_print_response::<Space>()?;
    opts.state
        .spaces
        .overwrite(&space.name, SpaceConfig::from(&space))?;
    delete_embedded_node(&opts, rpc.node_name()).await;
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
