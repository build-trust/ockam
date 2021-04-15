use ockam::{Context, Result, Route};
use ockam_transport_tcp::{TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect("127.0.0.1:4000").await?;

    // Send a message to the echoer worker, on a different node, over a tcp transport
    ctx.send(
        Route::new()
            .append_t(TCP, "127.0.0.1:4000")
            .append("echoer"),
        "Hello Ockam!".to_string()
    ).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("Initiator Received: {}", reply); // should print "Hello Ockam!"

    ctx.stop().await
}
