// This node starts a tcp listener and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::access_control::AllowedTransport;
use ockam::{Context, Mailboxes, Result, TcpTransport, TCP};
use std::sync::Arc;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:4000").await?;

    // Create an echoer worker
    ctx.start_worker_impl(
        Mailboxes::main("echoer", Arc::new(AllowedTransport::single(TCP))),
        Echoer,
    )
    .await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
