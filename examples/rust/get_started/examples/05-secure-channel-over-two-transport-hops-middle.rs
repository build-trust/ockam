// This node creates a tcp connection to a node at 127.0.0.1:4000
// Starts a forwarder worker to forward messages to 127.0.0.1:4000
// Starts a tcp listener at 127.0.0.1:3000
// It then runs forever waiting to route messages.

use hello_ockam::Forwarder;
use ockam::access_control::AllowAll;
use ockam::{Context, Result, TcpConnectionTrustOptions, TcpListenerTrustOptions, TcpTransport};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection to Bob.
    let connection_to_bob = tcp
        .connect("127.0.0.1:4000", TcpConnectionTrustOptions::insecure_test())
        .await?;

    // Start a Forwarder to forward messages to Bob using the TCP connection.
    ctx.start_worker("forward_to_bob", Forwarder(connection_to_bob), AllowAll, AllowAll)
        .await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:3000", TcpListenerTrustOptions::insecure_test())
        .await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
