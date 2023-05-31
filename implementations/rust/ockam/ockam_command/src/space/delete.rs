use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};

use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, RpcBuilder};
use crate::{docs, fmt_ok, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a space
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeleteCommand {
    /// Name of the space.
    #[arg(display_order = 1001)]
    pub name: String,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
) -> crate::Result<()> {
    let id = opts.state.spaces.get(&cmd.name)?.config().id.clone();

    let node_name = start_embedded_node(ctx, &opts, None).await?;
    let controller_route = &cmd.cloud_opts.route();

    // Send request
    let mut rpc = RpcBuilder::new(ctx, &opts, &node_name).build();
    rpc.request(api::space::delete(&id, controller_route))
        .await?;
    rpc.is_ok()?;

    // Remove from state
    let _ = opts.state.spaces.delete(&cmd.name);
    // TODO: remove projects associated to the space.
    //  Currently we are not storing that association in the project config file.

    delete_embedded_node(&opts, rpc.node_name()).await;

    // log the deletion
    opts.terminal
        .stdout()
        .plain(fmt_ok!("Space with name '{}' has been deleted.", &cmd.name))
        .machine(&cmd.name)
        .json(serde_json::json!({ "space": { "name": &cmd.name } }))
        .write_line()?;

    Ok(())
}
