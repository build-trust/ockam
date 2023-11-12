use clap::Args;
use colorful::Colorful;
use console::Term;

use ockam::Context;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::cloud::space::Spaces;
use ockam_api::cloud::Controller;
use ockam_api::nodes::InMemoryNode;

use crate::terminal::tui::DeleteCommandTui;
use crate::util::api::CloudOpts;
use crate::util::node_rpc;
use crate::{docs, fmt_ok, CommandGlobalOpts, Terminal, TerminalStream};

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a space
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeleteCommand {
    /// Name of the space.
    #[arg(display_order = 1001)]
    pub space_name: Option<String>,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    DeleteTui::run(ctx, opts, cmd).await
}

pub struct DeleteTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    controller: Controller,
    cmd: DeleteCommand,
}

impl DeleteTui {
    pub async fn run(
        ctx: Context,
        opts: CommandGlobalOpts,
        cmd: DeleteCommand,
    ) -> miette::Result<()> {
        let node = InMemoryNode::start(&ctx, &opts.state).await?;
        let controller = node.create_controller().await?;
        let tui = Self {
            ctx,
            opts,
            controller,
            cmd,
        };
        tui.delete().await
    }
}

#[ockam_core::async_trait]
impl DeleteCommandTui for DeleteTui {
    const ITEM_NAME: &'static str = "space";

    fn cmd_arg_item_name(&self) -> Option<&str> {
        self.cmd.space_name.as_deref()
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

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String> {
        let space_name = match &self.cmd.space_name {
            None => self.opts.state.spaces.default()?.name().to_string(),
            Some(n) => n.to_string(),
        };
        Ok(space_name)
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        Ok(self.opts.state.spaces.list_items_names()?)
    }

    async fn delete_single(&self, item_name: &str) -> miette::Result<()> {
        let space_id = self.opts.state.spaces.get(item_name)?.config().id.clone();
        self.controller.delete_space(&self.ctx, space_id).await?;
        let _ = self.opts.state.spaces.delete(item_name);

        self.terminal()
            .stdout()
            .plain(fmt_ok!(
                "The space with name {} has been deleted",
                item_name.light_magenta()
            ))
            .machine(item_name)
            .json(serde_json::json!({ "name": item_name }))
            .write_line()?;
        Ok(())
    }

    async fn delete_multiple(&self, items_names: Vec<String>) -> miette::Result<()> {
        for item_name in items_names {
            self.delete_single(&item_name).await?;
        }
        Ok(())
    }
}
