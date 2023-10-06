use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::cloud::space::Spaces;

use ockam_api::nodes::InMemoryNode;

use crate::util::api::CloudOpts;
use crate::util::node_rpc;
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

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, DeleteCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &Context,
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
) -> miette::Result<()> {
    if opts
        .terminal
        .confirmed_with_flag_or_prompt(cmd.yes, "Are you sure you want to delete this space?")?
    {
        let space_id = opts.state.spaces.get(&cmd.name)?.config().id.clone();
        let node = InMemoryNode::start(ctx, &opts.state).await?;
        let controller = node.create_controller().await?;
        controller.delete_space(ctx, space_id).await?;

        let _ = opts.state.spaces.delete(&cmd.name);
        // TODO: remove projects associated to the space.
        //  Currently we are not storing that association in the project config file.
        opts.terminal
            .stdout()
            .plain(fmt_ok!("Space with name '{}' has been deleted.", &cmd.name))
            .machine(&cmd.name)
            .json(serde_json::json!({ "space": { "name": &cmd.name } }))
            .write_line()?;
    }
    Ok(())
}
