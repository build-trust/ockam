use ockam::{route, stream::Stream, Context, Result, TcpTransport, TCP};
use ockam_get_started::Echoer;
use std::time::Duration;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect("127.0.0.1:4000").await?;

    // Create the stream client
    Stream::new(&ctx)?
        .stream_service("stream")
        .index_service("stream_index")
        .client_id("stream-over-cloud-node-initiator")
        .with_interval(Duration::from_millis(100))
        .connect(
            route![(TCP, "127.0.0.1:4000")],
            // Stream name from THIS node to the OTHER node
            "test-b-a",
            // Stream name from the OTHER node to THIS node
            "test-a-b",
        )
        .await?;

    // Start an echoer worker
    ctx.start_worker("echoer", Echoer).await?;

    Ok(())
}
