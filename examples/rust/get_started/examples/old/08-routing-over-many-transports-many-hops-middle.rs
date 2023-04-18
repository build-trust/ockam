// This node creates a tcp connection to a node at 127.0.0.1:4000
// Starts a ws listener at 127.0.0.1:3000
// It then runs forever waiting to route messages.

use ockam::{Context, Result, TcpTransport};
use ockam_transport_websocket::WebSocketTransport;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let node = node(ctx);
    let tcp = node.create_tcp_transport().await?;

    // Create a TCP connection
    tcp.connect("127.0.0.1:4000").await?;

    // Initialize the WS Transport.
    let ws = WebSocketTransport::create(node.context()).await?;

    // Create a WS listener and wait for incoming connections.
    ws.listen("127.0.0.1:3000").await?;

    // Don't call node.stop() here so this node runs forever.
    Ok(())
}
