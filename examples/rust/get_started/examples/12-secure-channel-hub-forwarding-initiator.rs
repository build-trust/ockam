use ockam::{Context, Result, Route, SecureChannel, Vault, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let hub = "Paste the address of the node you created on Ockam Hub here.";
    let secure_channel_forwarding_address =
        "Paste the forwarding address of the secure channel here.";

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(hub).await?;

    let vault = Vault::create(&ctx).await?;

    let channel = SecureChannel::create(
        &mut ctx,
        Route::new()
            .append_t(TCP, hub)
            .append(secure_channel_forwarding_address)
            .append("secure_channel"),
        &vault,
    )
    .await?;

    let echo_route = Route::new()
        .append(channel.address())
        .append("echo_service");

    ctx.send(echo_route, "Hello world!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("Initiator Received: {}", reply); // should print "Hello Ockam!"

    ctx.stop().await
}
