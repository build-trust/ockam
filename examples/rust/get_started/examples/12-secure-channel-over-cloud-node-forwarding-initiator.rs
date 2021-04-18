use ockam::{Context, Result, Route, SecureChannel, SoftwareVault, Vault};
use ockam_transport_tcp::{TcpTransport, TCP};

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

    let vault = Vault::create(&ctx, SoftwareVault::default()).await?;

    let channel = SecureChannel::create(
        &mut ctx,
        Route::new()
            .append_t(TCP, cloud_node_tcp_address)
            .append(secure_channel_listener_forwarding_address),
        &vault,
    )
    .await?;

    ctx.send(
        Route::new().append(channel.address()).append("echoer"),
        "Hello world!".to_string(),
    )
    .await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
