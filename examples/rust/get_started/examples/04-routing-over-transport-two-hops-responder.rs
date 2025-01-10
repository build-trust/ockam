// This node starts a tcp listener and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::tcp::{TcpListenerOptions, TcpTransportExtension};
use ockam::{node, Context, Result};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let node = node(ctx).await?;

    // Initialize the TCP Transport
    let tcp = node.create_tcp_transport()?;

    // Create an echoer worker
    node.start_worker("echoer", Echoer)?;

    // Create a TCP listener and wait for incoming connections.
    let listener = tcp.listen("127.0.0.1:4000", TcpListenerOptions::new()).await?;

    // Allow access to the Echoer via TCP connections from the TCP listener
    node.flow_controls()
        .add_consumer(&"echoer".into(), listener.flow_control_id());

    // Don't call node.shutdown() here so this node runs forever.
    Ok(())
}
