use channel_examples::client_worker::Client;
use ockam::{Result, SecureChannel};
use ockam_transport_tcp::TcpTransport;
use std::net::SocketAddr;
use std::str::FromStr;

#[ockam::node]
async fn main(ctx: ockam::Context) -> Result<()> {
    let hub_addr = SocketAddr::from_str("127.0.0.1:4000").unwrap();

    // Create and register a connection worker pair
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect("127.0.0.1:4000").await?;

    let channel = SecureChannel::create(
        &mut ctx,
        Route::new()
            .append_t(TCP, "127.0.0.1:4000")
            .append("40d60f6b")
            .append("secure_channel"),
    )
    .await?;

    let route = Route::new().append(channel.address()).append("echo_server");

    ctx.send(route, "Hello world!".to_string()).await?;

    let msg = ctx.receive::<String>().await?;
    println!("Received: {}", msg);

    Ok(())
}
