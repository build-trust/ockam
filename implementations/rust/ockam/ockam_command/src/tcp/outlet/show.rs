use async_trait::async_trait;
use core::fmt::Write;

use clap::Args;
use console::Term;
use miette::{miette, IntoDiagnostic};
use serde::Serialize;

use ockam::Context;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_api::{address::extract_address_value, nodes::models::portal::OutletStatus};
use ockam_core::api::Request;
use ockam_core::TryClone;
use ockam_multiaddr::MultiAddr;

use crate::tcp::util::alias_parser;
use crate::{docs, Command, CommandGlobalOpts};
use ockam_api::output::Output;

use crate::terminal::tui::ShowCommandTui;
use crate::tui::PluralTerm;

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

#[async_trait]
impl Command for ShowCommand {
    const NAME: &'static str = "tcp-outlet show";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        Ok(ShowTui::run(ctx.try_clone().into_diagnostic()?, opts, self.clone()).await?)
    }
}

#[derive(Debug, Serialize)]
struct OutletInformation {
    node_name: String,
    worker_address: MultiAddr,
    to: String,
}

impl Output for OutletInformation {
    fn item(&self) -> ockam_api::Result<String> {
        let mut w = String::new();
        write!(w, "Outlet")?;
        write!(w, "\n  On Node: {}", self.node_name)?;
        write!(w, "\n  From address: {}", self.worker_address)?;
        write!(w, "\n  To TCP server: {}", self.to)?;
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
        cmd.at = Some(node.node_name().to_string());

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
    const ITEM_NAME: PluralTerm = PluralTerm::TcpOutlet;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.cmd.alias.clone()
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
        let outlets: Vec<OutletStatus> = self
            .node
            .ask(&self.ctx, Request::get("/node/outlet"))
            .await?;
        let items_names: Vec<String> = outlets
            .into_iter()
            .map(|outlet| outlet.worker_addr.address().to_string())
            .collect();
        Ok(items_names)
    }

    async fn show_single(&self, item_name: &str) -> miette::Result<()> {
        let outlet_status: OutletStatus = self
            .node
            .ask(&self.ctx, Request::get(format!("/node/outlet/{item_name}")))
            .await?;
        let info = OutletInformation {
            node_name: self.node.node_name().to_string(),
            worker_address: outlet_status.worker_route().into_diagnostic()?,
            to: outlet_status.to.to_string(),
        };
        self.terminal()
            .stdout()
            .plain(info.item()?)
            .json_obj(info)?
            .write_line()?;
        Ok(())
    }
}
