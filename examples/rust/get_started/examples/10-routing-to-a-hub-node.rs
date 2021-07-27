// This program sends a message to the echo_service worker running on your node in Ockam Hub.

use ockam::{route, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Create a hub node by going to https://hub.ockam.network

    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>"; // e.g. "127.0.0.1:4000"

    // Initialize the TCP Transport.
    let _tcp = TcpTransport::create(&ctx).await?;

    // Send a message to the `echo_service` worker on your hub node.
    ctx.send(
        // route to the echo_service worker on your hub node
        route![(TCP, hub_node_tcp_address), "echo_service"],
        // the message you want echo-ed back
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Wait to receive the echo and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: '{}'", reply); // should print "Hello Ockam!"

    // Stop the node.
    ctx.stop().await
}
