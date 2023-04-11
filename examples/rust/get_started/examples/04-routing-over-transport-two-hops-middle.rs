// This node creates a tcp connection to a node at 127.0.0.1:4000
// Starts a forwarder worker to forward messages to 127.0.0.1:4000
// Starts a tcp listener at 127.0.0.1:3000
// It then runs forever waiting to route messages.

use hello_ockam::Forwarder;
use ockam::access_control::AllowAll;
use ockam::{Context, Result, TcpConnectionOptions, TcpListenerOptions, TcpTransport};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection to the responder node.
    let connection_to_responder = tcp.connect("127.0.0.1:4000", TcpConnectionOptions::new()).await?;

    // Create a Forwarder worker
    ctx.start_worker(
        "forward_to_responder",
        Forwarder(connection_to_responder),
        AllowAll,
        AllowAll,
    )
    .await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:3000", TcpListenerOptions::new()).await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
