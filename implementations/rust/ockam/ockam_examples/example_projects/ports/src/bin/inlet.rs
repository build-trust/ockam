use ockam::{route, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    println!("Hello, world!");

    let tcp = TcpTransport::create(&ctx).await?;

    tcp.create_inlet("127.0.0.1:1234", route![(TCP, "127.0.0.1:4321"), "outlet"])
        .await?;

    Ok(())
}
