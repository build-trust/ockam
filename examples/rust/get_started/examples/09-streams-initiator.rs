use ockam::{route, stream::Stream, try_route, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // Set the address of the Kafka node you created here. (e.g. "192.0.2.1:4000")
    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>";

    // Create a stream client
    let (sender, _receiver) = Stream::new(&ctx)
        .await?
        .stream_service("stream_kafka")
        .index_service("stream_kafka_index")
        .client_id("stream-over-cloud-node-initiator")
        .connect(
            route![(TCP, hub_node_tcp_address)], // route to hub
            "initiator-to-responder",            // outgoing stream
            "responder-to-initiator",            // incoming stream
        )
        .await?;

    // Send a message
    ctx.send(
        try_route![
            sender.clone(), // via the "initiator-to-responder" stream
            "echoer"        // to the "echoer" worker
        ]?,
        "Hello World!".to_string(),
    )
    .await?;

    // Receive a message from the "responder-to-initiator" stream
    let reply = ctx.receive_block::<String>().await?;
    println!("Reply via stream: {}", reply);

    ctx.stop().await
}
