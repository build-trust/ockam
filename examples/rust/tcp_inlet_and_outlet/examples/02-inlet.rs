use ockam::{route, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // We know that the Outlet node is listening for Ockam Routing Messages
    // over TCP and is running at Ockam Worker address "outlet".
    //
    // We assume the Outlet node is listening on port 4000, unless otherwise specified
    // by a second command line argument.

    let outlet_port = std::env::args().nth(2).unwrap_or("4000".to_string());
    let route_to_outlet = route![(TCP, &format!("127.0.0.1:{outlet_port}")), "outlet"];

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
    tcp.create_inlet(inlet_address, route_to_outlet).await?;

    // We won't call ctx.stop() here,
    // so this program will keep running until you interrupt it with Ctrl-C.
    Ok(())
}
