use ockam::{Context, Result, Route, TcpTransport};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP listener to receive Ockam Routing Messages from other ockam nodes.
    tcp.listen("127.0.0.1:4000").await?;

    // Wait to receive a message from an Ockam Node that is running a TCP Transport Outlet
    // at Ockam Worker address - "outlet".
    //
    // Return Route of that message, with a little modification, gives us route to the outlet
    // We replace the last hop address in return route - "app" with "outlet".
    //
    // The works because the message is sent to us from the main function of the node that
    // is running the outlet. Main functions have Ockam worker address "app". We replace it
    // with "outlet" to get route to our TCP Transport Outlet.

    let msg = ctx.receive::<String>().await?.take();
    let route_to_outlet: Route = msg.return_route().modify().pop_back().append("outlet").into();

    // Expect first command line argument to be the TCP address on which to start an Inlet
    // For example: 127.0.0.1:4001
    //
    // Create a TCP Transport Inlet that will listen on the given TCP address as a TCP server.
    //
    // The Inlet will:
    // 1. Wrap any raw TCP data it receives from a TCP client as payload of a new
    //    Ockam Routing Message. This Ockam Routing Message will have its onward_route
    //    be set to the route to a TCP Transport Outlet. This route_to_outlet is provided as
    //    the 2nd argument of the create_inlet() function.
    //
    // 2. Unwrap the payload of any Ockam Routing Message it receives back from the Outlet
    //    and send it as raw TCP data to q connected TCP client.

    let inlet_address = std::env::args().nth(1).expect("no inlet address given");
    tcp.create_inlet(inlet_address, route_to_outlet).await?;

    // We won't call ctx.stop() here,
    // so this program will keep running until you interrupt it with Ctrl-C.
    Ok(())
}
