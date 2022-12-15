// This node routes a message, to a worker on a different node, over the tcp transport.

use ockam::{route, Context, Result};
use ockam_transport_uds::{UdsTransport, UDS};
#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the UDS Transport.
    let uds = UdsTransport::create(&ctx).await?;

    let connection = uds.connect("/tmp/ockam-example-echoer").await;

    if let Err(e) = connection {
        println!("Error connecting to echoer {e}");
    }
    // Send a message to the "echoer" worker, on a different node, over a uds transport.
    let r = route![(UDS, "/tmp/ockam-example-echoer"), "echoer"];
    ctx.send(r, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
