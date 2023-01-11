use ockam::access_control::AllowAll;
use ockam::{
    authenticated_storage::InMemoryStorage,
    identity::{Identity, SecureChannelRegistry, TrustEveryonePolicy},
    remote::RemoteForwarder,
    vault::Vault,
    Context, Result, TcpTransport, TCP,
};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    let vault = Vault::create();
    let registry = SecureChannelRegistry::new();
    let e = Identity::create(&ctx, &vault).await?;
    let storage = InMemoryStorage::new();
    e.create_secure_channel_listener("secure_channel_listener", TrustEveryonePolicy, &storage, &registry)
        .await?;

    // Expect first command line argument to be the TCP address of a target TCP server.
    // For example: 127.0.0.1:4002
    //
    // Create a TCP Transport Outlet - at Ockam Worker address "outlet" -
    // that will connect, as a TCP client, to the target TCP server.
    //
    // This Outlet will:
    // 1. Unwrap the payload of any Ockam Routing Message that it receives from an Inlet
    //    and send it as raw TCP data to the target TCP server. First such message from
    //    an Inlet is used to remember the route back the Inlet.
    //
    // 2. Wrap any raw TCP data it receives, from the target TCP server,
    //    as payload of a new Ockam Routing Message. This Ockam Routing Message will have
    //    its onward_route be set to the route to an Inlet that is knows about because of
    //    a previous message from the Inlet.

    let outlet_target = std::env::args().nth(1).expect("no outlet target given");
    tcp.create_outlet("outlet", outlet_target, AllowAll).await?;

    // To allow Inlet Node and others to initiate an end-to-end secure channel with this program
    // we connect with 1.node.ockam.network:4000 as a TCP client and ask the forwarding
    // service on that node to create a forwarder for us.
    //
    // All messages that arrive at that forwarding address will be sent to this program
    // using the TCP connection we created as a client.
    let node_in_hub = (TCP, "1.node.ockam.network:4000");
    let forwarder = RemoteForwarder::create(&ctx, node_in_hub, AllowAll).await?;
    println!("\n[✓] RemoteForwarder was created on the node at: 1.node.ockam.network:4000");
    println!("Forwarding address in Hub is:");
    println!("{}", forwarder.remote_address());

    // We won't call ctx.stop() here,
    // so this program will keep running until you interrupt it with Ctrl-C.
    Ok(())
}
