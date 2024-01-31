use std::net::SocketAddr;

use clap::Args;
use core::fmt::Write;
use miette::{miette, IntoDiagnostic};
use serde::Serialize;

use ockam::{route, Context};
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::route_to_multiaddr;
use ockam_api::{
    address::extract_address_value,
    nodes::models::portal::{OutletList, OutletStatus},
};
use ockam_core::api::Request;
use ockam_core::AsyncTryClone;
use ockam_multiaddr::MultiAddr;

use crate::output::Output;
use crate::tcp::util::alias_parser;
use crate::terminal::tui::ShowCommandTui;
use crate::terminal::PluralTerm;
use crate::util::async_cmd;
use crate::Result;
use crate::{docs, CommandGlobalOpts, Term, Terminal, TerminalStream};

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show detailed information on TCP Outlets
#[derive(Clone, Debug, Args)]
#[command(
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ShowCommand {
    /// Show detailed information about the Outlet that has this alias. If you don't
    /// provide an alias, you will be prompted to select from a list of available Outlets
    /// to show
    #[arg(display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    pub alias: Option<String>,

    /// Show Outlet at the specified node. If you don't provide it, the default node will be used
    #[arg(long, display_order = 903, id = "NODE_NAME", value_parser = extract_address_value)]
    pub at: Option<String>,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "show tcp outlet".into()
    }

    pub async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        ShowTui::run(
            ctx.async_try_clone().await.into_diagnostic()?,
            opts,
            self.clone(),
        )
        .await
    }
}

#[derive(Debug, Serialize)]
struct OutletInformation {
    node_name: String,
    alias: String,
    addr: MultiAddr,
    socket_addr: SocketAddr,
}

impl Output for OutletInformation {
    fn output(&self) -> Result<String> {
        let mut w = String::new();
        write!(w, "Outlet")?;
        write!(w, "\n  On Node: {}", self.node_name)?;
        write!(w, "\n  Alias: {}", self.alias)?;
        write!(w, "\n  From Outlet: {}", self.addr)?;
        write!(w, "\n  To TCP: {}", self.socket_addr)?;
        Ok(w)
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
        ctx: Context,
        opts: CommandGlobalOpts,
        mut cmd: ShowCommand,
    ) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(&ctx, &opts.state, &cmd.at).await?;
        cmd.at = Some(node.node_name());

        let tui = Self {
            ctx,
            opts,
            cmd,
            node,
        };

        tui.show().await
    }
}

#[ockam_core::async_trait]
impl ShowCommandTui for ShowTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Outlet;

    fn cmd_arg_item_name(&self) -> Option<&str> {
        self.cmd.alias.as_deref()
    }

    fn node_name(&self) -> Option<&str> {
        self.cmd.at.as_deref()
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String> {
        self.cmd
            .alias
            .clone()
            .ok_or(miette!("No TCP Outlet alias provided"))
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        let outlets: OutletList = self
            .node
            .ask(&self.ctx, Request::get("/node/outlet"))
            .await?;
        let aliases: Vec<String> = outlets
            .list
            .into_iter()
            .map(|outlet| outlet.alias)
            .collect();
        Ok(aliases)
    }

    async fn show_single(&self, item_name: &str) -> miette::Result<()> {
        let outlet_status: OutletStatus = self
            .node
            .ask(&self.ctx, Request::get(format!("/node/outlet/{item_name}")))
            .await?;
        let info = OutletInformation {
            node_name: self.node.node_name().to_string(),
            alias: outlet_status.alias,
            addr: route_to_multiaddr(&route![outlet_status.worker_addr.to_string()])
                .ok_or_else(|| miette!("Invalid Outlet Address"))?,
            socket_addr: outlet_status.socket_addr,
        };
        self.terminal()
            .stdout()
            .plain(info.output()?)
            .json(serde_json::json!(info))
            .write_line()?;
        Ok(())
    }
}
