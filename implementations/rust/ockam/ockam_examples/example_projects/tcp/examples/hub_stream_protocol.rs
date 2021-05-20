use ockam::{stream::Stream, Context, Result, Route};
use ockam_transport_tcp::{TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let hub_addr = "127.0.0.1:4000";

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(hub_addr).await?;

    // Create 2 new stream workers
    let (tx, rx) = Stream::new(&ctx)?
        .connect(
            Route::new()
                .append_t(TCP, hub_addr)
                .append("stream_service"),
            "test-stream".to_string(),
        )
        .await?;

    // Send a message to the stream producer
    // ctx.send(tx, "Hello world!".to_string()).await?;

    // Get the next message from the stream consumer
    // let msg: String = rx.next().await;
    // println!("Forwarded: `{}`", msg);
    //ctx.stop().await

    Ok(())
}
