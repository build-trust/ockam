use ockam::{route, stream::Stream, Context, Result, TcpTransport, TCP};
use ockam_get_started::Echoer;
use std::time::Duration;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // Start a printer
    ctx.start_worker("echoer", Echoer).await?;

    // Create the stream
    Stream::new(&ctx)?
        .with_interval(Duration::from_millis(100))
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
