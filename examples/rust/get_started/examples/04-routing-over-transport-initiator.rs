// This node routes a message, to a worker on a different node, over the tcp transport.

use ockam::access_control::AllowedTransport;
use ockam::{route, Address, Context, Mailboxes, Result, TcpTransport, TCP};
use std::sync::Arc;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let _tcp = TcpTransport::create(&ctx).await?;

    // Send a message to the "echoer" worker, on a different node, over a tcp transport.
    let r = route![(TCP, "localhost:4000"), "echoer"];

    // FIXME: Child context is needed because "app" Context has LocalOriginOnly AccessControl,
    // that can be fixed by improving #[ockam::node] macro
    let mut child_ctx = ctx
        .new_context_impl(Mailboxes::main(
            Address::random_local(),
            Arc::new(AllowedTransport::single(TCP)),
        ))
        .await?;

    child_ctx.send(r, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = child_ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
