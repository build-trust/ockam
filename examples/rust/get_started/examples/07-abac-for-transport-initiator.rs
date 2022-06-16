// This node routes a message, to a worker on a different node, over the tcp transport.

use ockam::access_control::{AllowedTransport, LocalOriginOnly};
use ockam::{route, Context, Result, TcpTransport, TCP};

#[ockam::node(access_control = "LocalOriginOnly")]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let _tcp = TcpTransport::create(&ctx).await?;

    // A repeater Context is needed because the node Context has LocalOriginOnly AccessControl.
    let mut repeater_ctx = ctx.new_repeater(AllowedTransport::single(TCP)).await?;

    // Send a message to the "echoer" worker, on a different node, over a tcp transport.
    let r = route![(TCP, "localhost:4000"), "echoer"];
    repeater_ctx.send(r, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = repeater_ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
