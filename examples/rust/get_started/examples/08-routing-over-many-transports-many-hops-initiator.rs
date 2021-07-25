// This node routes a message, to a worker on a different node, over a ws and a tcp transport hops.

use ockam::{route, Context, Result, TCP};
use ockam_transport_websocket::{WebSocketTransport, WS};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the WS Transport.
    let ws = WebSocketTransport::create(&ctx).await?;

    // Create a WS connection.
    ws.connect("127.0.0.1:3000").await?;

    // Send a message to the "echoer" worker, on a different node, over two tcp hops.
    ctx.send(
        route![
            (WS, "127.0.0.1:3000"),  // middle node
            (TCP, "127.0.0.1:4000"), // responder node
            "echoer"
        ], // echoer worker on responder node
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
