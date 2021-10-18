// This node creates an end-to-end encrypted secure channel over two tcp transport hops.
// It then routes a message, to a worker on a different node, through this encrypted channel.

use ockam::{
    route, Address, Context, Entity, Result, TrustEveryonePolicy, Vault, TCP,
};
use ockam_transport_websocket::{WebSocketTransport, WS};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the WS Transport.
    let ws = WebSocketTransport::create(&ctx).await?;

    // Create a WS connection.
    ws.connect("127.0.0.1:3000").await?;

    let alice_vault = Vault::create(&ctx).expect("failed to create vault");
    let mut alice = Entity::create(&ctx, &alice_vault).await?;
    let middle: Address = (WS, "127.0.0.1:3000").into();
    let responder: Address = (TCP, "127.0.0.1:4000").into();
    let route = route![middle, responder, "bob_secure_channel_listener"];

    // Connect to a secure channel listener and perform a handshake.
    let channel = alice.create_secure_channel(route, TrustEveryonePolicy).await?;

    // Send a message to the echoer worker via the channel.
    let echoer_route = route![channel, "echoer"];

    ctx.send(echoer_route, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"
                                         // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
