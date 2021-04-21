use ockam::{Context, Result, TcpTransport};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen("127.0.0.1:4000").await?;
    tcp.connect("127.0.0.1:6000").await?;

    // This node never shuts down.
    Ok(())
}
