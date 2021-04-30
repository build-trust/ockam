use ockam::{Context, Profile, Result, Route, TcpTransport, Vault, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Create a cloud node by going to https://hub.ockam.network
    let cloud_node_tcp_address = "Paste the tcp address of your cloud node here.";

    let secure_channel_listener_forwarding_address =
        "Paste the forwarding address of the secure channel here.";

    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection to your cloud node.
    tcp.connect(cloud_node_tcp_address).await?;

    let vault = Vault::create(&ctx)?;

    let mut alice = Profile::create(&ctx, &vault)?;

    let channel = alice
        .create_secure_channel(
            &ctx,
            Route::new()
                .append_t(TCP, cloud_node_tcp_address)
                .append(secure_channel_listener_forwarding_address),
        )
        .await?;

    ctx.send(
        Route::new().append(channel).append("echoer"),
        "Hello world!".to_string(),
    )
    .await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
