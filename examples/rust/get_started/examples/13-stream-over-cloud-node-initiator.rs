/// This example uses the stream service to send messages between two
/// clients.  A stream is a buffered message sending channel, which
/// means that you can run `initiator` and `responder` in any order
/// you like.
use ockam::{stream::Stream, Context, Result, Route, TcpTransport, TCP};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect("127.0.0.1:4000").await?;

    let (tx, _) = Stream::new(&ctx)?
        .connect(
            Route::new().append_t(TCP, "127.0.0.1:4000"),
            "test_stream".to_string(),
        )
        .await?;

    ctx.send(tx, "Hello World!".to_string()).await?;

    Ok(())
}
