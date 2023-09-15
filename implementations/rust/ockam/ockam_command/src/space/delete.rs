use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::cloud::space::Spaces;

use crate::node::util::{delete_embedded_node, start_node_manager};
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

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
) -> miette::Result<()> {
    if opts
        .terminal
        .confirmed_with_flag_or_prompt(cmd.yes, "Are you sure you want to delete this space?")?
    {
        let space_id = opts.state.spaces.get(&cmd.name)?.config().id.clone();
        let node_manager = start_node_manager(ctx, &opts, None).await?;
        let controller = node_manager
            .make_controller_client()
            .await
            .into_diagnostic()?;

        controller
            .delete_space(ctx, space_id)
            .await
            .into_diagnostic()?;

        let _ = opts.state.spaces.delete(&cmd.name);
        // TODO: remove projects associated to the space.
        //  Currently we are not storing that association in the project config file.
        delete_embedded_node(&opts, &node_manager.node_name()).await;

        opts.terminal
            .stdout()
            .plain(fmt_ok!("Space with name '{}' has been deleted.", &cmd.name))
            .machine(&cmd.name)
            .json(serde_json::json!({ "space": { "name": &cmd.name } }))
            .write_line()?;
    }
    Ok(())
}
