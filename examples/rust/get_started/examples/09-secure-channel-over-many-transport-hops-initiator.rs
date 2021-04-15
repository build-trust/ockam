use ockam::{Context, Result, Route, SecureChannel};
use ockam_transport_tcp::{TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect("127.0.0.1:4000").await?;

    let route_to_listener = Route::new()
        // Send a message to node B
        .append_t(TCP, "127.0.0.1:4000")
        // Send a message to node C
        .append_t(TCP, "127.0.0.1:6000")
        .append("secure_channel_listener");
    let channel = SecureChannel::create(&mut ctx, route_to_listener).await?;

    // Send a message to the echoer worker via the channel.
    ctx.send(
        Route::new().append(channel.address()).append("echoer"),
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("Initiator Received: {}", reply); // should print "Hello Ockam!"

    ctx.stop().await
}
