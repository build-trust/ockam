use ockam::{route, Context, Result, TcpTransport, TCP};
use ockam::{Entity, SecureChannels, TrustEveryonePolicy, Vault};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a Vault to store our cryptographic keys and an Entity to represent this Node.
    // Then initiate a handshake with the secure channel listener on the node that has the
    // TCP Transport Inlet.
    //
    // For this example, we know that the Inlet node is listening for Ockam Routing Messages
    // over TCP at "2.node.ockam.network:4000" and its secure channel listener is
    // at address: "secure_channel_listener_service".

    let vault = Vault::create(&ctx)?;
    let mut client = Entity::create(&ctx, &vault)?;
    let r = route![(TCP, "2.node.ockam.network:4000"), "secure_channel_listener_service"];
    let channel = client.create_secure_channel(r, TrustEveryonePolicy)?;

    // Expect first command line argument to be the TCP address of a target TCP server.
    // For example: 127.0.0.1:5000
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
    tcp.create_outlet("outlet", outlet_target).await?;

    // Send an Ockam Routing Message, via the secure channel, to the node that is
    // running a TCP Transport Inlet.
    //
    // For this example we know that the node at "2.node.ockam.network:4000" has a worker
    // at address "tcp_inlet_service" that can create dedicated Inlets for us.
    //
    // To request a dedicated Inlet, we send the "tcp_inlet_service" a message, via our
    // secure channel. The body of this message must be the local worker address of the
    // Outlet we want to pair with our new Inlet.

    let route_to_tcp_inlet_service = route![channel, "tcp_inlet_service"];
    ctx.send(route_to_tcp_inlet_service, "outlet".to_string()).await?;

    // The "tcp_inlet_service" replies with the port number of our dedicated Inlet
    // on 2.node.ockam.network.

    let port = ctx.receive::<i32>().await?.take().body();
    println!("Inlet is accessible on - 2.node.ockam.network:{}", port);

    // We won't call ctx.stop() here,
    // so this program will keep running until you interrupt it with Ctrl-C.
    Ok(())
}
