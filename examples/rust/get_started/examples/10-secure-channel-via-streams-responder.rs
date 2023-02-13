use hello_ockam::Echoer;
use ockam::access_control::AllowAll;
use ockam::identity::{Identity, TrustEveryonePolicy};
use ockam::{route, stream::Stream, vault::Vault, Context, Result, TcpTransport};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;

    // Set the address of the Kafka node you created here. (e.g. "192.0.2.1:4000")
    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>";
    let hub_node_address = tcp.connect(hub_node_tcp_address).await?;

    // Create a vault
    let vault = Vault::create();

    // Create an Identity
    let bob = Identity::create(&ctx, &vault).await?;

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
            route![hub_node_address],    // route to hub
            "sc-responder-to-initiator", // outgoing stream
            "sc-initiator-to-responder", // incoming stream
        )
        .await?;

    // Start an echoer worker
    ctx.start_worker("echoer", Echoer, AllowAll, AllowAll).await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
