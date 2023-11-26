use std::collections::HashMap;
use std::net::SocketAddr;

use clap::Args;
use core::fmt::Write;
use miette::miette;
use ockam_api::cli_state::StateDirTrait;
use serde::Serialize;

use ockam::{route, Context};
use ockam_api::address::extract_address_value;
use ockam_api::nodes::models::portal::OutletStatus;
use ockam_api::nodes::BackgroundNode;
use ockam_api::route_to_multiaddr;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::output::Output;
use crate::tcp::outlet::list::send_request;
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
    outlet_aliases: Vec<String>,
    aliases_node: HashMap<String, BackgroundNode>,
}

impl ShowTui {
    pub async fn run(
        ctx: Context,
        opts: CommandGlobalOpts,
        cmd: ShowCommand,
    ) -> miette::Result<()> {
        let mut running_node_names: Vec<String> = Vec::new();

        if cmd.alias.is_some() {
            let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
            let node_name = extract_address_value(&node_name)?;

            if opts.state.nodes.get(&node_name)?.is_running() {
                running_node_names.push(node_name);
            }
        } else {
            // Build the running nodes list when no alias is provided.
            running_node_names = opts
                .state
                .nodes
                .list_items_names()?
                .iter()
                .filter_map(|node_name| opts.state.nodes.get(node_name).ok())
                .filter(|node| node.is_running())
                .filter_map(|node| extract_address_value(node.name()).ok())
                .collect();
        }

        if running_node_names.is_empty() {
            return Err(miette!("No running nodes found"));
        }

        let mut outlet_aliases: Vec<String> = Vec::new();
        let mut aliases_node: HashMap<String, BackgroundNode> = HashMap::new();

        for node_name in running_node_names {
            let res = send_request(&ctx, &opts, node_name.clone()).await;
            match res {
                Ok(outlets) => {
                    let mut node_outlets: Vec<String> = outlets
                        .list
                        .iter()
                        .map(|outlet| outlet.alias.clone())
                        .collect();

                    let node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;
                    node_outlets.iter().for_each(|alias| {
                        aliases_node.insert(alias.to_string(), node.clone());
                    });
                    outlet_aliases.append(&mut node_outlets);
                }
                Err(_) => continue,
            }
        }

        let tui = Self {
            ctx,
            opts,
            cmd,
            outlet_aliases,
            aliases_node,
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
        Ok(self.outlet_aliases.clone())
    }

    async fn show_single(&self, item_name: &str) -> miette::Result<()> {
        match self.aliases_node.get(item_name) {
            Some(node) => {
                let node_name = node.node_name().clone();
                let outlet_status: OutletStatus = node
                    .ask(&self.ctx, make_api_request(item_name.to_string())?)
                    .await?;
                let info = OutletInformation {
                    node_name: node_name.to_string(),
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
            }
            _ => return Err(miette!("No running nodes found")),
        }
        Ok(())
    }

    async fn show_multiple(&self, aliases: Vec<String>) -> miette::Result<()> {
        for alias in aliases {
            self.show_single(&alias).await?;
        }
        Ok(())
    }
}

/// Construct a request to show a tcp outlet
fn make_api_request(alias: String) -> Result<Request> {
    let request = Request::get(format!("/node/outlet/{alias}"));
    Ok(request)
}
