use ockam::{Context, Result, Route, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let remote_node = "127.0.0.1:10222";

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(remote_node).await?;

    ctx.send(
        Route::new()
            .append_t(TCP, remote_node)
            .append("echo_service"),
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Then wait for a message back!
    let msg = ctx.receive::<String>().await?;
    println!("Received echo: '{}'", msg);
    ctx.stop().await
}
