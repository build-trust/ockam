// This node creates an end-to-end encrypted secure channel over two tcp transport hops.
// It then routes a message, to a worker on a different node, through this encrypted channel.

use ockam::{Context, Profile, Result, Route, TcpTransport, Vault, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection.
    tcp.connect("127.0.0.1:3000").await?;

    let vault = Vault::create(&ctx)?;

    let mut alice = Profile::create(&ctx, &vault)?;

    // Connect to a secure channel listener and perform a handshake.
    let channel = alice
        .create_secure_channel(
            &ctx,
            // route to the secure channel listener
            Route::new()
                .append_t(TCP, "127.0.0.1:3000") // middle node
                .append_t(TCP, "127.0.0.1:4000") // responder node
                .append("secure_channel_listener"), // secure_channel_listener on responder node,
        )
        .await?;

    // Send a message to the echoer worker via the channel.
    ctx.send(
        Route::new().append(channel).append("echoer"),
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
