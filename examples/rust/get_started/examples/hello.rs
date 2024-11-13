use ockam::identity::{SecureChannelListenerOptions, SecureChannelOptions};
use ockam::{node, route, Context, Result};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let mut node = node(ctx).await?;
    // Create an Identity to represent Bob
    let bob = node.create_identity().await?;

    // Create a secure channel listener for Bob that will wait for requests to
    // initiate an Authenticated Key Exchange.
    let options = SecureChannelListenerOptions::new();
    let sc_flow_control_id = options.spawner_flow_control_id();
    node.create_secure_channel_listener(&bob, "bob", options)?;

    // Create an entity to represent Alice.
    let alice = node.create_identity().await?;

    // As Alice, connect to Bob's secure channel listener and perform an
    // Authenticated Key Exchange to establish an encrypted secure channel with Bob.
    let channel = node
        .create_secure_channel(&alice, "bob", SecureChannelOptions::new())
        .await?;

    // Send a message, ** THROUGH ** the secure channel,
    // to the "app" worker on the other side.
    //
    // This message will automatically get encrypted when it enters the channel
    // and decrypted just before it exits the channel.
    node.flow_controls().add_consumer(&"app".into(), &sc_flow_control_id);
    node.send(route![channel, "app"], "Hello Ockam!".to_string()).await?;

    // Wait to receive a message for the "app" worker and print it.
    let message = node.receive::<String>().await?;
    println!("App Received: {}", message.into_body()?); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    node.shutdown().await
}
