// This node starts a tcp listener and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::access_control::AllowAll;
use ockam::{node, Context, Result, TcpListenerOptions};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let node = node(ctx);
    let tcp = node.create_tcp_transport().await?;

    // Create an echoer worker
    node.start_worker("echoer", Echoer, AllowAll, AllowAll).await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:4000", TcpListenerOptions::new()).await?;

    // Don't call node.stop() here so this node runs forever.
    Ok(())
}
