use ockam::{Context, Result, Route, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let hub = "Paste the address of the node you created on Ockam Hub here.";
    let echo_service =
        "Paste the forwarded address that the server received from registration here.";

    let tcp = TcpTransport::create(&ctx).await?;

    tcp.connect(hub).await?;

    ctx.send(
        Route::new().append_t(TCP, hub).append(echo_service),
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Then wait for a message back!
    let msg = ctx.receive::<String>().await?;
    println!("Received echo: '{}'", msg);
    ctx.stop().await
}
