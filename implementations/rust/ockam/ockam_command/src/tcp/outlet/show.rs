use std::net::SocketAddr;

use clap::Args;
use core::fmt::Write;
use miette::miette;
use serde::Serialize;

use ockam::{route, Context};
use ockam_api::nodes::models::portal::OutletStatus;
use ockam_api::nodes::BackgroundNode;
use ockam_api::route_to_multiaddr;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use crate::node::NodeOpts;
use crate::output::Output;
use crate::tcp::util::alias_parser;
use crate::util::node_rpc;
use crate::Result;
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show a TCP Outlet's details
#[derive(Clone, Debug, Args)]
#[command(
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ShowCommand {
    /// Name assigned to outlet that will be shown
    #[arg(display_order = 900, required = true, id = "ALIAS", value_parser = alias_parser)]
    alias: String,

    /// Node from the outlet that is to be shown. If none are provided, the default node will be used
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self))
    }
}

#[derive(Debug, Serialize)]
struct OutletInformation {
    alias: String,
    addr: MultiAddr,
    socket_addr: SocketAddr,
}

impl Output for OutletInformation {
    fn output(&self) -> Result<String> {
        let mut w = String::new();
        write!(w, "Outlet")?;
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
    let node = BackgroundNode::create(&ctx, &opts.state, &cmd.node_opts.at_node).await?;
    let outlet_status: OutletStatus = node
        .ask(&ctx, Request::get(format!("/node/outlet/{}", cmd.alias)))
        .await?;
    let info = OutletInformation {
        alias: outlet_status.alias,
        addr: route_to_multiaddr(&route![outlet_status.worker_addr.to_string()])
            .ok_or_else(|| miette!("Invalid Outlet Address"))?,
        socket_addr: outlet_status.socket_addr,
    };

    opts.terminal
        .stdout()
        .plain(info.output()?)
        .json(serde_json::json!(info))
        .write_line()?;
    Ok(())
}
