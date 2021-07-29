use ockam::{route, stream::Stream, Context, Result, TcpTransport, TCP};
use ockam_get_started::Echoer;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>"; // e.g. "127.0.0.1:4000"

    // Create the stream client
    Stream::new(&ctx)?
        .stream_service("stream")
        .index_service("stream_index")
        .client_id("stream-over-cloud-node-initiator")
        .with_interval(Duration::from_millis(100))
        .connect(
            route![(TCP, hub_node_tcp_address)],
            "responder-to-initiator",
            "initiator-to-responder",
        )
        .await?;

    // Start an echoer worker
    ctx.start_worker("echoer", Echoer).await?;

    Ok(())
}
