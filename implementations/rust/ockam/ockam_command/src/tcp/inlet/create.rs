use crate::node::default_node_name;
use crate::tcp::util::alias_parser;
use crate::util::{
    bind_to_port_check, exitcode, extract_address_value, node_rpc, process_nodes_multiaddr,
    RpcBuilder,
};
use crate::{help, CommandGlobalOpts, Result};

use anyhow::anyhow;
use clap::Args;
use ockam::identity::IdentityIdentifier;
use ockam::{Context, TcpTransport};
use ockam_api::nodes::models::portal::CreateInlet;
use ockam_api::nodes::models::portal::InletStatus;
use ockam_core::api::Request;
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{MultiAddr, Protocol as _};
use std::net::SocketAddr;

const HELP_DETAIL: &str = include_str!("../../constants/tcp/inlet/help_detail.txt");

/// Create TCP Inlets
#[derive(Clone, Debug, Args)]
#[command(after_long_help = help::template(HELP_DETAIL))]
pub struct CreateCommand {
    /// Node on which to start the tcp inlet.
    #[arg(long, display_order = 900, id = "NODE", default_value_t = default_node_name())]
    at: String,

    /// Address on which to accept tcp connections.
    #[arg(long, display_order = 900, id = "SOCKET_ADDRESS")]
    from: SocketAddr,

    /// Route to a tcp outlet.
    #[arg(long, display_order = 900, id = "ROUTE")]
    to: MultiAddr,

    /// Authorized identity for secure channel connection
    #[arg(long, name = "AUTHORIZED", display_order = 900)]
    authorized: Option<IdentityIdentifier>,

    /// Assign a name to this inlet.
    #[arg(long, display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    alias: Option<String>,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, mut cmd): (CommandGlobalOpts, CreateCommand)) -> Result<()> {
    cmd.to = process_nodes_multiaddr(&cmd.to, &opts.state)?;

    // Check if the port is used by some other services or process
    if !bind_to_port_check(&cmd.from) {
        return Err(crate::error::Error::new(
            exitcode::IOERR,
            anyhow!("Another process is listening on the provided port!"),
        ));
    }

    let tcp = TcpTransport::create(&ctx).await?;
    let node = extract_address_value(&cmd.at)?;

    let req = {
        let mut payload = if cmd.to.matches(0, &[Project::CODE.into()]) {
            if cmd.authorized.is_some() {
                return Err(anyhow!("--authorized can not be used with project addresses").into());
            }
            CreateInlet::via_project(cmd.from, cmd.to)
        } else {
            CreateInlet::to_node(cmd.from, cmd.to, cmd.authorized)
        };
        if let Some(a) = cmd.alias {
            payload.set_alias(a)
        }
        Request::post("/node/inlet").body(payload)
    };

    let mut rpc = RpcBuilder::new(&ctx, &opts, &node).tcp(&tcp)?.build();
    rpc.request(req).await?;
    rpc.parse_response::<InletStatus>()?;

    Ok(())
}
