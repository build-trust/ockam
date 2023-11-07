use crate::terminal::ConfirmResult;
use crate::util::node_rpc;
use crate::{fmt_ok, CommandGlobalOpts};
use clap::Args;
use colorful::Colorful;
use miette::miette;
use ockam_api::cli_state::{CliState, StateDirTrait, StateItemTrait};
use ockam_api::cloud::space::Spaces;
use ockam_api::nodes::InMemoryNode;
use ockam_node::Context;

/// Removes the local Ockam configuration including all Identities and Nodes
#[derive(Clone, Debug, Args)]
pub struct ResetCommand {
    /// Confirm the reset without prompting
    #[arg(long, short)]
    yes: bool,

    /// Remove your spaces from the Orchestrator
    #[arg(long)]
    with_orchestrator: bool,
}

impl ResetCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, ResetCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(ctx: &Context, opts: CommandGlobalOpts, cmd: ResetCommand) -> miette::Result<()> {
    let delete_orchestrator_resources = cmd.with_orchestrator && opts.state.is_enrolled()?;
    if !cmd.yes {
        let msg = if delete_orchestrator_resources {
            "This will delete the local Ockam configuration and remove your spaces from the Orchestrator. Are you sure?"
        } else {
            "This will delete the local Ockam configuration. Are you sure?"
        };
        match opts.terminal.confirm(msg)? {
            ConfirmResult::Yes => {}
            ConfirmResult::No => {
                return Ok(());
            }
            ConfirmResult::NonTTY => {
                return Err(miette!("Use --yes to confirm"));
            }
        }
    }

    if delete_orchestrator_resources {
        let spinner = opts.terminal.progress_spinner();
        if let Some(ref s) = spinner {
            s.set_message("Deleting spaces from the Orchestrator...")
        }
        let node = InMemoryNode::start(ctx, &opts.state).await?;
        let controller = node.create_controller().await?;
        for space in opts.state.spaces.list()? {
            if let Some(ref s) = spinner {
                s.set_message(format!("Deleting space '{}'...", space.name()))
            }
            controller
                .delete_space(ctx, space.config().id.clone())
                .await?;
            let _ = opts.state.spaces.delete(space.name());
        }
        if let Some(ref s) = spinner {
            s.finish_and_clear();
        }
        opts.terminal
            .write_line(fmt_ok!("Orchestrator spaces deleted"))?;
    }
    CliState::delete()?;
    opts.terminal
        .stdout()
        .plain(fmt_ok!("Local Ockam configuration deleted"))
        .write_line()?;
    Ok(())
}
