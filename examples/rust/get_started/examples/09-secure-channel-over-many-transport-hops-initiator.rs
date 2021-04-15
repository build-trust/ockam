use ockam::{Context, Result, Route, SecureChannel};
use ockam_transport_tcp::{TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect("127.0.0.1:4000").await?;

    let route_to_listener =
        Route::new()
            .append_t(TCP, "127.0.0.1:4000") // middle node
            .append_t(TCP, "127.0.0.1:6000") // responder node
            .append("secure_channel_listener"); // secure_channel_listener on responder node
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
