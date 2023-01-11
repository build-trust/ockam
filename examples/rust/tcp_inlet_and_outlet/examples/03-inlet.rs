use ockam::access_control::AllowAll;
use ockam::authenticated_storage::InMemoryStorage;
use ockam::identity::{Identity, SecureChannelRegistry, TrustEveryonePolicy};
use ockam::{route, vault::Vault, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a Vault to store our cryptographic keys and an Identity to represent this Node.
    // Then initiate a handshake with the secure channel listener on the node that has the
    // TCP Transport Outlet.
    //
    // For this example, we know that the Outlet node is listening for Ockam Routing Messages
    // over TCP and its secure channel listener is at address: "secure_channel_listener".
    //
    // We assume the Outlet node is listening on port 4000, unless otherwise specified
    // by a second command line argument.

    let vault = Vault::create();
    let registry = SecureChannelRegistry::new();
    let e = Identity::create(&ctx, &vault).await?;
    let outlet_port = std::env::args().nth(2).unwrap_or_else(|| "4000".to_string());
    let r = route![(TCP, &format!("127.0.0.1:{outlet_port}")), "secure_channel_listener"];
    let storage = InMemoryStorage::new();
    let channel = e
        .create_secure_channel(r, TrustEveryonePolicy, &storage, &registry)
        .await?;

    // We know Secure Channel address that tunnels messages to the node with an Outlet,
    // we also now that Outlet lives at "outlet" address at that node.

    let route_to_outlet = route![channel, "outlet"];

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
    tcp.create_inlet(inlet_address, route_to_outlet, AllowAll).await?;

    // We won't call ctx.stop() here,
    // so this program will keep running until you interrupt it with Ctrl-C.
    Ok(())
}
