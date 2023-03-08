// This node starts a tcp listener and an echoer worker.
// It then runs forever waiting for messages.

use abac_examples::Echoer;
use ockam::access_control::{AllowedTransport, LocalOriginOnly};
use ockam::{Context, Result, TcpTransport, WorkerBuilder, TCP};

#[ockam::node(access_control = "LocalOriginOnly")]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let node = node(ctx);
    let tcp = node.create_tcp_transport().await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:4000").await?;

    // Create an echoer worker
    node.start_worker("echoer", Echoer, AllowedTransport::single(TCP), AllowAll).await?;

    // Don't call node.stop() here so this node runs forever.
    Ok(())
}
