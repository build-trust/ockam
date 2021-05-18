// This node creates an end-to-end encrypted secure channel over two tcp transport hops.
// It then routes a message, to a worker on a different node, through this encrypted channel.

use ockam::{Address, Context, LocalEntity, Result, Route, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection.
    tcp.connect("127.0.0.1:3000").await?;

    let mut initiator = LocalEntity::create(&ctx, "initiator").await?;
    let middle: Address = (TCP, "127.0.0.1:3000").into();
    let responder: Address = (TCP, "127.0.0.1:4000").into();
    let route: Route = vec![middle, responder, "secure_channel_listener".into()].into();

    // Connect to a secure channel listener and perform a handshake.
    let channel = initiator.create_secure_channel(route).await?;

    // Send a message to the echoer worker via the channel.
    let echoer_route: Route = vec![channel, "echoer".into()].into();

    ctx.send(echoer_route, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
