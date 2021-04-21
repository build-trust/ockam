use ockam::{Context, Result, Route, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let hub = "Paste the address of the node you created on Ockam Hub here.";

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(hub).await?;

    ctx.send(
        Route::new().append_t(TCP, hub).append("echo_service"),
        "Hello Ockam!".to_string(),
    )
    .await?;

    let msg = ctx.receive::<String>().await?;
    println!("Received return message: '{}'", msg);
    ctx.stop().await
}
