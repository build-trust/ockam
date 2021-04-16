use ockam::{Result, Route, SecureChannel};
use ockam_transport_tcp::{TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: ockam::Context) -> Result<()> {
    let hub_addr = "104.42.24.183:4000";

    // Create and register a connection worker pair
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(hub_addr).await?;

    let channel = SecureChannel::create(
        &mut ctx,
        Route::new()
            .append_t(TCP, hub_addr)
            .append("26289ddb")
            .append("secure_channel"),
    )
    .await?;

    let route = Route::new().append(channel.address()).append("echo_server");

    ctx.send(route, "Hello world!".to_string()).await?;

    let msg = ctx.receive::<String>().await?;
    println!("Received: {}", msg);

    Ok(())
}
