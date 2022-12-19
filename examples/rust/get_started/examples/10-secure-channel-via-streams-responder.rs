use hello_ockam::Echoer;
use ockam::access_control::AllowAll;
use ockam::{channel::SecureChannel, route, stream::Stream, vault::Vault, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // Set the address of the Kafka node you created here. (e.g. "192.0.2.1:4000")
    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>";

    // Create a vault
    let vault = Vault::create();

    // Create a secure channel listener at address "secure_channel_listener"
    SecureChannel::create_listener(&ctx, "secure_channel_listener", &vault).await?;

    // Create a stream client
    Stream::new(&ctx)
        .await?
        .stream_service("stream_kafka")
        .index_service("stream_kafka_index")
        .client_id("secure-channel-over-stream-over-cloud-node-responder")
        .connect(
            route![(TCP, hub_node_tcp_address)], // route to hub
            "sc-responder-to-initiator",         // outgoing stream
            "sc-initiator-to-responder",         // incoming stream
        )
        .await?;

    // Start an echoer worker
    ctx.start_worker("echoer", Echoer, AllowAll, AllowAll).await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
