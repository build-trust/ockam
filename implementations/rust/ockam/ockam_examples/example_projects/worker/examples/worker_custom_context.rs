//! This example demonstrates how you can gain access to the worker
//! context, without having to conform to a usual worker lifecycle.

use ockam::{Context, Result};
use std::time::Duration;
use tracing::info;

/// A custom runner with access to the node context
struct Custom(Context);

impl Custom {
    fn run(self) {
        let mut ctx = self.0;
        tokio::spawn(async move {
            ctx.send("app", "Hello 1".to_string()).await.unwrap();
            tokio::time::sleep(Duration::from_millis(500)).await;

            // Wait for a reply
            let reply = ctx.receive::<String>().await.unwrap();
            info!("Ok: {}", reply);

            tokio::time::sleep(Duration::from_millis(500)).await;
            ctx.send("app", "Hello 3".to_string()).await.unwrap();
        });
    }
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Create and run our non-worker
    let ctx2 = ctx.new_detached("some.address").await?;
    Custom(ctx2).run();

    assert_eq!(ctx.receive::<String>().await?, "Hello 1".to_string());
    info!("Ok");

    ctx.send("some.address", "Hello 2".to_string()).await?;

    assert_eq!(ctx.receive::<String>().await?, "Hello 3".to_string());
    info!("Ok");

    ctx.stop().await
}
