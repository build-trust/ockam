use crate::node::NodeOpts;
use crate::tui::{PluralTerm, ShowCommandTui};
use crate::{docs, Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use console::Term;
use miette::miette;
use ockam::Context;
use ockam_api::nodes::models::transport::TransportStatus;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::output::Output;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_core::api::Request;
use ockam_core::TryClone;
use std::net::SocketAddr;
use std::str::FromStr;

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show a TCP Connection
#[derive(Clone, Debug, Args)]
#[command(
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ShowCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    /// TCP Connection worker address or socket address
    pub address: Option<String>,
}

#[async_trait]
impl Command for ShowCommand {
    const NAME: &'static str = "tcp-connection show";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        Ok(ShowTui::run(ctx, opts, self.clone()).await?)
    }
}

pub struct ShowTui {
    pub ctx: Context,
    pub opts: CommandGlobalOpts,
    pub cmd: ShowCommand,
    pub node: BackgroundNodeClient,
}

impl ShowTui {
    pub async fn run(
        ctx: &Context,
        opts: CommandGlobalOpts,
        mut cmd: ShowCommand,
    ) -> miette::Result<()> {
        let node =
            BackgroundNodeClient::create(ctx, opts.state.clone(), &cmd.node_opts.at_node).await?;
        cmd.node_opts.at_node = Some(node.node_name().to_string());

        let tui = Self {
            ctx: ctx.try_clone()?,
            opts,
            cmd,
            node,
        };

        tui.show().await
    }
}

#[async_trait]
impl ShowCommandTui for ShowTui {
    const ITEM_NAME: PluralTerm = PluralTerm::TcpConnection;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.cmd.address.clone()
    }

    fn node_name(&self) -> Option<&str> {
        self.cmd.node_opts.at_node.as_deref()
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String> {
        self.cmd
            .address
            .clone()
            .ok_or(miette!("No TCP Connection address provided"))
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

    async fn show_single(&self, item_name: &str) -> miette::Result<()> {
        let transport_status: TransportStatus = self
            .node
            .ask(
                &self.ctx,
                Request::get(format!("/node/tcp/connection/{item_name}")),
            )
            .await?;
        self.terminal()
            .to_stdout()
            .plain(transport_status.item()?)
            .json_obj(&transport_status)?
            .machine(&transport_status.worker_address)
            .write_line()?;
        Ok(())
    }
}
