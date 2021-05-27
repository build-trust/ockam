use ockam::{stream::Stream, Context, Result, Route, Routed, TcpTransport, Worker, TCP};
use std::time::Duration;

struct Printer;

#[ockam::worker]
impl Worker for Printer {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, _: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("Message received: {}", msg);
        Ok(())
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect("127.0.0.1:4000").await?;

    // Start a printer
    ctx.start_worker("printer", Printer).await?;

    // Create the stream
    Stream::new(&ctx)?
        .with_interval(Duration::from_millis(100))
        .connect(
            Route::new().append_t(TCP, "127.0.0.1:4000"),
            // Stream name from THIS to OTHER
            "test-b-a",
            // Stream name from OTHER to THIS
            "test-a-b",
        )
        .await?;
    Ok(())
}
