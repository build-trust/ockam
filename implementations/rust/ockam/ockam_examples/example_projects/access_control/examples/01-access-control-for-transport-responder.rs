// This node starts a tcp listener and an echoer worker.
// It then runs forever waiting for messages.

use abac_examples::Echoer;
use ockam::access_control::{AllowedTransport, LocalOriginOnly};
use ockam::{Context, Result, TcpTransport, WorkerBuilder, TCP};

#[ockam::node(access_control = "LocalOriginOnly")]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:4000").await?;

    // Create an echoer worker
    WorkerBuilder::with_access_control(AllowedTransport::single(TCP), "echoer", Echoer)
        .start(&ctx)
        .await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
