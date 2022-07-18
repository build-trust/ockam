// This node routes a message, to a worker on a different node, over two tcp transport hops.

use ockam::{route, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let _tcp = TcpTransport::create(&ctx).await?;

    // Send a message to the "echoer" worker, on a different node, over two tcp hops.
    // Use ports 3000 & 4000, unless otherwise specified by command line arguments.
    let port_middle = std::env::args().nth(1).unwrap_or_else(|| "3000".to_string());
    let port_responder = std::env::args().nth(2).unwrap_or_else(|| "4000".to_string());
    let r = route![
        (TCP, &format!("localhost:{port_middle}")),
        (TCP, &format!("localhost:{port_responder}")),
        "echoer"
    ];
    ctx.send(r, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
