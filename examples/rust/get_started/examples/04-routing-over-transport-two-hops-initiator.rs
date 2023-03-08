// This node routes a message, to a worker on a different node, over two tcp transport hops.

use ockam::{node, route, Context, Result, TcpConnectionOptions};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let mut node = node(ctx);
    let tcp = node.create_tcp_transport().await?;

    // Create a TCP connection to the middle node.
    let connection_to_middle_node = tcp.connect("localhost:3000", TcpConnectionOptions::new()).await?;

    // Send a message to the "echoer" worker, on a different node, over two tcp hops.
    let r = route![connection_to_middle_node, "forward_to_responder", "echoer"];
    node.send(r, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = node.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    node.stop().await
}
