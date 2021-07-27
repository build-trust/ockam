// This node routes a message, to a different node, using a forwarding address on the hub node.
use ockam::{route, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Create a hub node by going to https://hub.ockam.network
    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>"; // e.g. "127.0.0.1:4000"

    // Run 11-connecting-devices-using-hub-node-responder,
    // it will print the forwarding address of echoer on your hub node
    let echoer_forwarding_address = "<Address copied from responder output>";

    // Initialize the TCP Transport.
    let _tcp = TcpTransport::create(&ctx).await?;

    // Send a message to the echoer worker, on a different node,
    // using a forwarding address on your hub node
    ctx.send(
        route![(TCP, hub_node_tcp_address), echoer_forwarding_address],
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
