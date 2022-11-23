// This node routes a message, to a worker on a different node, over the tcp transport.

use ockam::access_control::{AllowFromTransport, AllowToTransport};
use ockam::{route, Context, Mailboxes, Result, TcpTransport, TCP};
use std::sync::Arc;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let _tcp = TcpTransport::create(&ctx).await?;

    let mailboxes = Mailboxes::main(
        "main",
        Arc::new(AllowFromTransport::single(TCP)),
        Arc::new(AllowToTransport::single(TCP)),
    );
    let mut child_ctx = ctx.new_detached_with_mailboxes(mailboxes).await?;

    // Send a message to the "echoer" worker, on a different node, over a tcp transport.
    // Use port 4000, unless otherwise specified by command line argument.
    let port = std::env::args().nth(1).unwrap_or_else(|| "4000".to_string());
    let r = route![(TCP, &format!("localhost:{port}")), "echoer"];
    child_ctx.send(r, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = child_ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
