use crate::node::{get_node_name, initialize_node_if_default};
use crate::util::Rpc;
use crate::util::{node_rpc, parse_node_name};
use crate::CommandGlobalOpts;
use clap::Args;
use ockam_api::nodes::models;
use ockam_api::nodes::models::transport::CreateTcpListener;
use ockam_core::api::Request;
use ockam_multiaddr::proto::{DnsAddr, Tcp};
use ockam_multiaddr::MultiAddr;

#[derive(Args, Clone, Debug)]
pub struct CreateCommand {
    /// Node at which to create the listener
    #[arg(global = true, long, value_name = "NODE")]
    pub at: Option<String>,

    /// Address for this listener (eg. 127.0.0.1:7000)
    pub address: String,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.at);
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.at);
    let node_name = parse_node_name(&node_name)?;
    let mut rpc = Rpc::background(&ctx, &opts, &node_name)?;
    rpc.request(Request::post("/node/tcp/listener").body(CreateTcpListener::new(cmd.address)))
        .await?;
    let response = rpc.parse_response::<models::transport::TransportStatus>()?;

    let socket = response.socket_addr()?;
    let port = socket.port();
    let mut multiaddr = MultiAddr::default();
    multiaddr.push_back(DnsAddr::new("localhost"))?;
    multiaddr.push_back(Tcp::new(port))?;
    println!("Tcp listener created! You can send messages to it via this route:\n`{multiaddr}`");

    Ok(())
}
