use crate::node::default_node_name;
use crate::util::extract_address_value;
use crate::util::node_rpc;
use crate::util::Rpc;
use crate::CommandGlobalOpts;
use clap::Args;
use ockam_api::nodes::models;
use ockam_api::nodes::models::transport::{CreateTransport, TransportMode, TransportType};
use ockam_core::api::Request;
use ockam_multiaddr::proto::{DnsAddr, Service, Tcp};
use ockam_multiaddr::MultiAddr;

#[derive(Args, Clone, Debug)]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: TCPListenerNodeOpts,

    /// Address for this listener (eg. 127.0.0.1:7000)
    pub address: String,
}

#[derive(Clone, Debug, Args)]
pub struct TCPListenerNodeOpts {
    /// Node at which to create the listener
    #[arg(global = true, long, value_name = "NODE", default_value_t = default_node_name())]
    pub at: String,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let at_node_name = &cmd.node_opts.at;
    let node_name = extract_address_value(at_node_name)?;
    let mut rpc = Rpc::background(&ctx, &opts, &node_name)?;
    rpc.request(
        Request::post("/node/tcp/listener").body(CreateTransport::new(
            TransportType::Tcp,
            TransportMode::Listen,
            cmd.address,
        )),
    )
    .await?;
    let response = rpc.parse_response::<models::transport::TransportStatus>()?;

    let port = opts
        .state
        .nodes
        .get(&node_name)?
        .setup()?
        .default_tcp_listener()?
        .addr
        .port();
    let mut multiaddr = MultiAddr::default();
    multiaddr.push_back(DnsAddr::new("localhost"))?;
    multiaddr.push_back(Tcp::new(port))?;
    multiaddr.push_back(Service::new(response.worker_addr.to_string()))?;
    println!("Tcp listener created! You can send messages to it via this route:\n`{multiaddr}`",);

    Ok(())
}
