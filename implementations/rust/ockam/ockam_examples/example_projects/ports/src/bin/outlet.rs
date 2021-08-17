use ockam::{Context, Result, TcpTransport};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    println!("Hello, world!");

    let tcp = TcpTransport::create(&ctx).await?;

    tcp.listen("127.0.0.1:4321").await?;
    tcp.create_outlet("outlet", "127.0.0.1:22").await?;

    Ok(())
}
