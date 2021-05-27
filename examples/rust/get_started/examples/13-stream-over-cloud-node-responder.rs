use ockam::{stream::Stream, Context, Result, Route, TcpTransport, TCP};
use std::time::Duration;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect("127.0.0.1:4000").await?;

    let (tx, mut rx) = Stream::new(&ctx)?
        .with_interval(Duration::from_millis(100))
        .connect(
            Route::new().append_t(TCP, "127.0.0.1:4000"),
            // Stream name from THIS to OTHER
            "test-b-a",
            // Stream name from OTHER to THIS
            "test-a-b",
        )
        .await?;

    let msg = rx.next::<String>().await?;
    println!("Sent from peer: {}", msg);

    ctx.stop().await
}
