use ockam::identity::SecureChannelOptions;
use ockam::{node, route, Context, Result, Route, TcpConnectionOptions, TcpInletOptions};
use ockam_transport_tcp::TcpTransportExtension;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let node = node(ctx);
    let tcp = node.create_tcp_transport().await?;

    // Create a Vault to store our cryptographic keys and an Identity to represent this Node.
    // Then initiate a handshake with the secure channel listener on the node that has the
    // TCP Transport Outlet.
    //
    // For this example, we know that the Outlet node is listening for Ockam Routing Messages
    // through a Remote Relay at "1.node.ockam.network:4000" and its forwarder address
    // points to secure channel listener.
    let e = node.create_identity().await?;

    // Expect second command line argument to be the Outlet node forwarder address
    let forwarding_address = std::env::args().nth(2).expect("no outlet forwarding address given");
    let node_in_hub = tcp
        .connect("1.node.ockam.network:4000", TcpConnectionOptions::new())
        .await?;
    let r = route![node_in_hub, forwarding_address, "secure_channel_listener"];
    let channel = node.create_secure_channel(&e, r, SecureChannelOptions::new()).await?;

    // We know Secure Channel address that tunnels messages to the node with an Outlet,
    // we also now that Outlet lives at "outlet" address at that node.
    let route_to_outlet: Route = route![channel, "outlet"];

    // Expect first command line argument to be the TCP address on which to start an Inlet
    // For example: 127.0.0.1:4001
    //
    // Create a TCP Transport Inlet that will listen on the given TCP address as a TCP server.
    //
    // The Inlet will:
    // 1. Wrap any raw TCP data it receives from a TCP client as payload of a new
    //    Ockam Routing Message. This Ockam Routing Message will have its onward_route
    //    be set to the route to a TCP Transport Outlet. This route_to_outlet is provided as
    //    the second argument of the create_inlet() function.
    //
    // 2. Unwrap the payload of any Ockam Routing Message it receives back from the Outlet
    //    and send it as raw TCP data to q connected TCP client.

    let inlet_address = std::env::args().nth(1).expect("no inlet address given");
    tcp.create_inlet(inlet_address, route_to_outlet, TcpInletOptions::new())
        .await?;

    // We won't call ctx.stop() here,
    // so this program will keep running until you interrupt it with Ctrl-C.
    Ok(())
}
