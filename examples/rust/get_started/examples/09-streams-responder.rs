use hello_ockam::Echoer;
use ockam::access_control::AllowAll;
use ockam::{node, route, Context, Result, TcpConnectionOptions};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let node = node(ctx);
    let tcp = node.create_tcp_transport().await?;

    // Start an echoer worker
    node.start_worker("echoer", Echoer, AllowAll, AllowAll).await?;

    // Set the address of the Kafka node you created here. (e.g. "192.0.2.1:4000")
    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>";
    let node_in_hub = tcp.connect(hub_node_tcp_address, TcpConnectionOptions::new()).await?;

    // Create a stream client
    node.create_stream()
        .await?
        .stream_service("stream_kafka")
        .index_service("stream_kafka_index")
        .client_id("stream-over-cloud-node-initiator")
        .connect(
            route![node_in_hub],      // route to hub
            "responder-to-initiator", // outgoing stream
            "initiator-to-responder", // incoming stream
        )
        .await?;

    // Don't call node.stop() here so this node runs forever.
    Ok(())
}
