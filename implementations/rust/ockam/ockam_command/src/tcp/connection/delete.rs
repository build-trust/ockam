use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use console::Term;
use ockam_api::fmt_ok;
use std::net::SocketAddr;
use std::str::FromStr;

use ockam_api::nodes::{models, BackgroundNodeClient};
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_core::api::Request;
use ockam_core::TryClone;
use ockam_node::Context;

use crate::tui::{DeleteCommandTui, PluralTerm};
use crate::{docs, node::NodeOpts, Command, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a TCP Connection
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// TCP Connection worker address or socket address
    pub address: Option<String>,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,

    /// Delete all the TCP Connections
    #[arg(long)]
    all: bool,
}

#[async_trait]
impl Command for DeleteCommand {
    const NAME: &'static str = "tcp-connection delete";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        Ok(DeleteTui::run(ctx, opts, self).await?)
    }
}

#[derive(TryClone)]
struct DeleteTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    node: BackgroundNodeClient,
    cmd: DeleteCommand,
}

impl DeleteTui {
    pub async fn run(
        ctx: &Context,
        opts: CommandGlobalOpts,
        cmd: DeleteCommand,
    ) -> miette::Result<()> {
        let node =
            BackgroundNodeClient::create(ctx, opts.state.clone(), &cmd.node_opts.at_node).await?;
        let tui = Self {
            ctx: ctx.try_clone()?,
            opts,
            node,
            cmd,
        };
        tui.delete().await
    }
}

#[async_trait]
impl DeleteCommandTui for DeleteTui {
    const ITEM_NAME: PluralTerm = PluralTerm::TcpConnection;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.cmd.address.clone()
    }

    fn cmd_arg_delete_all(&self) -> bool {
        self.cmd.all
    }

    fn cmd_arg_confirm_deletion(&self) -> bool {
        self.cmd.yes
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        let transports = super::ListCommand::get_connection_list(&self.ctx, &self.node).await?;
        let is_socket_address = self
            .cmd
            .address
            .as_ref()
            .map(|a| SocketAddr::from_str(a).is_ok())
            .unwrap_or(false);
        Ok(transports
            .into_iter()
            .map(|t| {
                if is_socket_address {
                    t.socket_address
                } else {
                    t.worker_address
                }
            })
            .collect())
    }

    async fn delete_single(&self, item_name: &str) -> miette::Result<()> {
        let req = Request::delete("/node/tcp/connection").body(
            models::transport::DeleteTransport::new(item_name.to_string()),
        );
        self.node.tell(&self.ctx, req).await?;
        self.terminal()
            .to_stdout()
            .plain(fmt_ok!(
                "TCP Connection {item_name} has been successfully deleted"
            ))
            .machine(item_name)
            .write_line()?;
        Ok(())
    }
}
