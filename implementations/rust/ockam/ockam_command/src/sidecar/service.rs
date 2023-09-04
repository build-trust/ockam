
use clap::Args;
use ockam::identity::identities;
use ockam::{TcpListenerOptions, node, TcpTransportExtension};
use ockam_api::echoer::Echoer;
use ockam_api::identity::IdentityServiceV2;
use ockam_node::Context;
use std::net::{SocketAddr, IpAddr, Ipv4Addr};

use crate::util::embedded_node_that_is_not_stopped;
use crate::util::parsers::socket_addr_parser;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/secure_relay_inlet/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/secure_relay_inlet/after_long_help.txt");

/// Create and setup a new stateless sidecar
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct SidecarService {
    /// Address on which to accept tcp connections.
    #[arg(long, display_order = 900, id = "SOCKET_ADDRESS", default_value_t = default_addr(), value_parser = socket_addr_parser)]
    from: SocketAddr,

    /// Just print the recipe and exit
    #[arg(long)]
    dry_run: bool,

}
pub(crate) fn default_addr() -> SocketAddr {
    let port = 5000;
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port)
}

impl SidecarService {
    pub fn run(self, opts: CommandGlobalOpts) {
        embedded_node_that_is_not_stopped(start_sidecar, (opts, self));
    }
}
async fn start_sidecar(
    ctx: Context,
    args: (CommandGlobalOpts, SidecarService),
) -> miette::Result<()> {

    // Create a node with default implementations
    let node = node(ctx);

    // Initialize the TCP Transport.
    let tcp = node.create_tcp_transport().await.unwrap();

    node.start_worker("echoer", Echoer).await.unwrap();

    let identities = identities::identities();
    let worker = IdentityServiceV2::new(identities).await.unwrap(); 
    node.start_worker("identity_service", worker).await.unwrap();

    // Create a TCP listener and wait for incoming connections.
    let listener = tcp.listen(args.1.from.to_string(), TcpListenerOptions::new()).await.unwrap();

    // Allow access to the Echoer 
    node.flow_controls()
        .add_consumer("echoer", listener.flow_control_id());

    node.flow_controls()
        .add_consumer("identity_service", listener.flow_control_id());

    // Don't call node.stop() here so this node runs forever.
    Ok(())
}