use ockam::{route, stream::Stream, Context, Result, TcpTransport, TCP};
use hello_ockam::Echoer;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let node = node(ctx);
    let _tcp = node.create_tcp_transport().await?;

    // Start an echoer
    node.start_worker("echoer", Echoer).await?;

    // Create the stream
    node.create_stream().await?
        .connect(
            route![(TCP, "localhost:4000")],
            // Stream name from THIS to OTHER
            "test-b-a",
            // Stream name from OTHER to THIS
            "test-a-b",
        )
        .await?;
    Ok(())
}
