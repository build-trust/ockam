use ockam::{Context, Result, Route, TcpTransport};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Listen for incoming message from the other node with running Outlet
    tcp.listen("127.0.0.1:1234").await?;

    // Receive message from the other node with outlet address in the body
    let msg = ctx.receive::<String>().await?.take();
    // This is return route to a worker that sent us message
    let mut return_route = msg.return_route();
    // This is String with outlet address, that other node sent to us
    let outlet_address = msg.body();
    // This is route to the Outlet on the other node
    let outlet_route: Route = return_route
        .modify()
        .pop_back()
        .append(outlet_address)
        .into();

    // This is inlet listening address
    let inlet_address = "127.0.0.1:5000";
    // Let's create inlet that will listen on 127.0.0.1 port 5000, and stream messages
    // to the Outlet on the other node
    tcp.create_inlet(inlet_address, outlet_route.clone())
        .await?;

    println!(
        "Inlet created on: {} with outlet route: {}",
        inlet_address, outlet_route
    );

    // We won't call ctx.stop() here, this program will run until you stop it with Ctrl-C
    Ok(())
}
