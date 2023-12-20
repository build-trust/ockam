use std::net::SocketAddr;

use clap::Args;
use core::fmt::Write;
use miette::miette;
use ockam_api::cli_state::StateDirTrait;
use serde::Serialize;

use ockam::{route, Context};
use ockam_api::address::extract_address_value;
use ockam_api::nodes::models::portal::{OutletList, OutletStatus};
use ockam_api::nodes::BackgroundNode;
use ockam_api::route_to_multiaddr;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::output::Output;
use crate::tcp::util::alias_parser;
use crate::terminal::tui::ShowCommandTui;
use crate::util::node_rpc;
use crate::Result;
use crate::{docs, CommandGlobalOpts, Term, Terminal, TerminalStream};

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show a TCP Outlet's details
#[derive(Clone, Debug, Args)]
#[command(
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ShowCommand {
    /// Name assigned to outlet that will be shown
    #[arg(display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    alias: Option<String>,

    /// Node from the outlet that is to be shown. If none are provided, the default node will be used
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        node_rpc(run_impl, (opts, self))
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

pub async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> miette::Result<()> {
    ShowTui::run(ctx, opts, cmd).await
}

pub struct ShowTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    cmd: ShowCommand,
    node: BackgroundNode,
}

impl ShowTui {
    pub async fn run(
        ctx: Context,
        opts: CommandGlobalOpts,
        cmd: ShowCommand,
    ) -> miette::Result<()> {
        let node_name = {
            let name = get_node_name(&opts.state, &cmd.node_opts.at_node);
            extract_address_value(&name)?
        };
        if !opts.state.nodes.get(&node_name)?.is_running() {
            return Err(miette!("The node '{}' is not running", node_name));
        }
        let node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;

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
    const ITEM_NAME: &'static str = "TCP Outlet";

    fn cmd_arg_item_name(&self) -> Option<&str> {
        self.cmd.alias.as_deref()
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
            .ask(&self.ctx, make_api_request(item_name.to_string())?)
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

    async fn show_multiple(&self, aliases: Vec<String>) -> miette::Result<()> {
        let outlets: OutletList = self
            .node
            .ask(&self.ctx, Request::get("/node/outlet"))
            .await?;
        let node_name = self.node.node_name();
        let plain = self.terminal().build_list(
            &outlets.list,
            &format!("TCP Outlets on Node {node_name}"),
            &format!("No TCP Outlets found on Node {node_name}."),
        )?;

        let json: Vec<_> = outlets
            .list
            .iter()
            .filter(|outlet| aliases.contains(&outlet.alias))
            .map(|outlet| {
                Ok(serde_json::json!({
                    "alias": outlet.alias,
                    "from": outlet.worker_address()?,
                    "to": outlet.socket_addr,
                }))
            })
            .flat_map(|res: miette::Result<_, ockam_core::Error>| res.ok())
            .collect();

        self.terminal()
            .stdout()
            .plain(plain)
            .json(serde_json::json!(json))
            .write_line()?;
        Ok(())
    }
}

/// Construct a request to show a tcp outlet
fn make_api_request(alias: String) -> Result<Request> {
    let request = Request::get(format!("/node/outlet/{alias}"));
    Ok(request)
}
