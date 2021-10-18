use hello_ockam::Echoer;
use ockam::{route, stream::Stream, Context, Entity, Result, TcpTransport, TrustEveryonePolicy, Vault, TCP};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // Set the address of the Kafka node you created here. (e.g. "192.0.2.1:4000")
    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>";

    // Create a vault
    let vault = Vault::create(&ctx).await?;
    let mut bob = Entity::create(&ctx, &vault).await?;

    // Create a secure channel listener at address "secure_channel_listener"
    bob.create_secure_channel_listener("secure_channel_listener", TrustEveryonePolicy)
        .await?;

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
    ctx.start_worker("echoer", Echoer).await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
