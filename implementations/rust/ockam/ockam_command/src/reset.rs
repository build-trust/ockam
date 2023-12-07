use clap::Args;
use colorful::Colorful;
use miette::{miette, WrapErr};
use ockam_api::cloud::space::Spaces;

use ockam_api::nodes::InMemoryNode;
use ockam_node::Context;

use crate::terminal::ConfirmResult;
use crate::util::node_rpc;
use crate::{color, fmt_ok, CommandGlobalOpts, OckamColor};

/// Removes the local Ockam configuration including all Identities and Nodes
#[derive(Clone, Debug, Args)]
pub struct ResetCommand {
    /// Confirm the reset without prompting
    #[arg(long, short)]
    yes: bool,

    /// Remove your spaces from the Orchestrator
    #[arg(long)]
    all: bool,
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
    let delete_orchestrator_resources =
        cmd.all && opts.state.is_enrolled().await.unwrap_or_default();
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
        if let Err(e) = delete_orchestrator_resources_impl(ctx, opts.clone()).await {
            match opts.terminal.confirm(
                "We couldn't delete the resources from the Orchestrator. Do you want to continue?",
            )? {
                ConfirmResult::Yes => {}
                _ => {
                    return Err(e);
                }
            }
        }
    }
    opts.state.reset().await?;
    opts.terminal
        .stdout()
        .plain(fmt_ok!("Local Ockam configuration deleted"))
        .write_line()?;
    Ok(())
}

async fn delete_orchestrator_resources_impl(
    ctx: &Context,
    opts: CommandGlobalOpts,
) -> miette::Result<()> {
    let node = InMemoryNode::start(ctx, &opts.state).await?;
    let spaces = node
        .get_spaces(ctx)
        .await
        .wrap_err("Failed to retrieve spaces from the Orchestrator")?;
    if spaces.is_empty() {
        return Ok(());
    }
    let spinner = opts.terminal.progress_spinner();
    if let Some(s) = spinner.as_ref() {
        s.set_message("Deleting spaces from the Orchestrator..")
    };
    for space in spaces {
        if let Some(s) = spinner.as_ref() {
            s.set_message(format!(
                "Deleting space {}...",
                color!(space.name, OckamColor::PrimaryResource)
            ))
        };
        node.delete_space(ctx, &space.id).await?;
        if let Some(s) = spinner.as_ref() {
            s.set_message(format!(
                "Space {} deleted from the Orchestrator",
                color!(space.name, OckamColor::PrimaryResource)
            ))
        };
    }
    if let Some(s) = spinner {
        s.finish_with_message("Orchestrator spaces deleted")
    }
    Ok(())
}
