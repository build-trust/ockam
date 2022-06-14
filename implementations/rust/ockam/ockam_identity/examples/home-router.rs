use ockam_core::Result;
use ockam_node::Context;
use ockam_transport_tcp::TcpTransport;

async fn async_main(ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen("127.0.0.1:4141").await?;
    Ok(())
}

fn main() -> Result<()> {
    let (ctx, mut exe) = ockam_node::NodeBuilder::without_access_control().build();
    exe.execute(async move {
        async_main(ctx).await.unwrap();
    })
    .unwrap();
    Ok(())
}
