use ockam::{Context, Result, Route, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let hub = "Paste the address of the node you created on Ockam Hub here.";
    let echo_service_forwarding_address = "Paste the forwarding address of the echo_service here.";

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(hub).await?;

    // Send a message to the echoer worker, on a different node, over a tcp transport
    ctx.send(
        Route::new()
            .append_t(TCP, hub)
            .append(echo_service_forwarding_address),
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("Initiator Received: {}", reply); // should print "Hello Ockam!"

    ctx.stop().await
}
