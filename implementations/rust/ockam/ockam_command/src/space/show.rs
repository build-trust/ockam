use clap::Args;

use ockam::Context;
use ockam_api::cli_state::{SpaceConfig, StateDirTrait, StateItemTrait};
use ockam_api::cloud::space::Space;

use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, RpcBuilder};
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show the details of a space
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    /// Name of the space.
    #[arg(display_order = 1001)]
    pub name: String,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, ShowCommand)) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: ShowCommand,
) -> crate::Result<()> {
    let id = opts.state.spaces.get(&cmd.name)?.config().id.clone();

    let node_name = start_embedded_node(ctx, &opts, None).await?;
    let controller_route = &cmd.cloud_opts.route();

    // Send request
    let mut rpc = RpcBuilder::new(ctx, &opts, &node_name).build();
    rpc.request(api::space::show(&id, controller_route)).await?;
    let space = rpc.parse_and_print_response::<Space>()?;
    opts.state
        .spaces
        .overwrite(&cmd.name, SpaceConfig::from(&space))?;
    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
