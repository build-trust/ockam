use ockam::{route, stream::Stream, Context, Result, TcpTransport, TCP};
use hello_ockam::Echoer;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // Start an echoer
    ctx.start_worker("echoer", Echoer).await?;

    // Create the stream
    Stream::new(&ctx).await?
        .connect(
            route![(TCP, "localhost:4000")],
            // Stream name from THIS to OTHER
            "test-b-a",
            // Stream name from OTHER to THIS
            "test-a-b",
        )
        .await?;
    Ok(())
}
