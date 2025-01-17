use clap::Args;
use colorful::Colorful;
use miette::{miette, WrapErr};
use tracing::error;

use crate::CommandGlobalOpts;
use ockam_api::colors::OckamColor;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::space::Spaces;
use ockam_api::terminal::notification::NotificationHandler;
use ockam_api::terminal::ConfirmResult;
use ockam_api::{color, fmt_ok, CliState};
use ockam_node::Context;

use crate::docs;
use crate::util::async_cmd;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");
const UNSAFE_TAG: &str = include_str!("../static/unsafe_tag.txt");

#[derive(Clone, Debug, Args)]
#[command(
before_help = docs::before_help(UNSAFE_TAG),
about = docs::about("Removes the local Ockam configuration including all Identities and Nodes"),
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ResetCommand {
    /// Confirm the reset without prompting
    #[arg(long, short)]
    yes: bool,

    /// Remove your spaces from the Orchestrator
    #[arg(long)]
    all: bool,
}

// TODO: Detach all eBPFs
impl ResetCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "reset".into()
    }

    pub fn hard_reset(&self) {
        if let Err(err) = CliState::hard_reset() {
            error!("Failed to hard reset CliState, err={err}");
        }
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let delete_orchestrator_resources =
            self.all && opts.state.is_enrolled().await.unwrap_or_default();
        if !self.yes {
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
        {
            let _notification_handler =
                NotificationHandler::start(&opts.state, opts.terminal.clone());
            opts.state.reset().await?;
        }

        #[cfg(privileged_portals_support)]
        ockam::tcp::TcpTransport::detach_all_ockam_ebpfs_globally();

        opts.terminal
            .stdout()
            .plain(fmt_ok!("Local Ockam configuration deleted"))
            .write_line()?;
        Ok(())
    }
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
    let pb = opts.terminal.spinner();
    if let Some(s) = pb.as_ref() {
        s.set_message("Deleting spaces from the Orchestrator..")
    };
    for space in spaces {
        if let Some(s) = pb.as_ref() {
            s.set_message(format!(
                "Deleting space {}...",
                color!(space.name, OckamColor::PrimaryResource)
            ))
        };
        node.delete_space(ctx, &space.id).await?;
        if let Some(s) = pb.as_ref() {
            s.set_message(format!(
                "Space {} deleted from the Orchestrator",
                color!(space.name, OckamColor::PrimaryResource)
            ))
        };
    }
    if let Some(s) = pb {
        s.finish_with_message("Orchestrator spaces deleted")
    }
    Ok(())
}
