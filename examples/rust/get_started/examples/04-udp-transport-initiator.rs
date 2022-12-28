// This node routes a message, to a worker on a different node, over the udp transport.

use ockam::{route, Context, Result};
use ockam_transport_udp::{UdpTransport, UDP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the UDP Transport.
    let _udp = UdpTransport::create(&ctx).await?;

    // Send a message to the "echoer" worker, on a different node, over an udp transport.
    // Wait to receive a reply and print it.
    // Use port 4000, unless otherwise specified by command line argument.
    let port = std::env::args().nth(1).unwrap_or_else(|| "4000".to_string());
    let r = route![(UDP, format!("localhost:{port}")), "echoer"];
    ctx.send(r, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // sohuld print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
