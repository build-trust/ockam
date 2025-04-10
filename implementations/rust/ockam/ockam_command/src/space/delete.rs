use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use console::Term;
use std::sync::Arc;

use crate::node_command::InMemoryNodeCommand;
use crate::shared_args::IdentityOpts;
use crate::terminal::tui::DeleteCommandTui;
use crate::tui::PluralTerm;
use crate::{docs, Command, CommandGlobalOpts};
use ockam::Context;
use ockam_api::colors::OckamColor;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::space::Spaces;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_api::{color, fmt_ok};
use ockam_core::TryClone;

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a space
#[derive(Clone, Debug, Args)]
#[command(
hide = true,
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeleteCommand {
    /// Name of the space.
    #[arg(display_order = 1001)]
    pub space_name: Option<String>,

    #[command(flatten)]
    pub identity_opts: IdentityOpts,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

#[async_trait]
impl Command for DeleteCommand {
    const NAME: &'static str = "space delete";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        DeleteNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}

#[derive(Clone)]
struct DeleteNodeCommand {
    opts: CommandGlobalOpts,
    command: DeleteCommand,
}

impl DeleteNodeCommand {
    pub fn new(opts: CommandGlobalOpts, command: DeleteCommand) -> Self {
        Self { opts, command }
    }
}

#[async_trait]
impl InMemoryNodeCommand for DeleteNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let tui = DeleteTui {
            opts: self.opts.clone(),
            cmd: self.command.clone(),
            node,
        };
        tui.delete().await
    }
}

#[derive(TryClone)]
pub struct DeleteTui {
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
    node: Arc<InMemoryNode>,
}

#[ockam_core::async_trait]
impl DeleteCommandTui for DeleteTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Space;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.cmd.space_name.clone()
    }

    fn cmd_arg_delete_all(&self) -> bool {
        false
    }

    fn cmd_arg_confirm_deletion(&self) -> bool {
        self.cmd.yes
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        Ok(self
            .opts
            .state
            .get_spaces()
            .await?
            .iter()
            .map(|s| s.space_name())
            .collect())
    }

    async fn delete_single(&self, item_name: &str) -> miette::Result<()> {
        self.node.delete_space_by_name(item_name).await?;

        self.terminal()
            .to_stdout()
            .plain(fmt_ok!(
                "The space with name {} has been deleted",
                color!(item_name, OckamColor::PrimaryResource)
            ))
            .machine(item_name)
            .json(serde_json::json!({ "name": item_name }))
            .write_line()?;
        Ok(())
    }
}
