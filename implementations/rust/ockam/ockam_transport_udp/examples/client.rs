use ockam::route;
use ockam_core::Result;
use ockam_node::Context;
use ockam_transport_udp::{UdpTransport, UDP};

#[ockam_macros::node]
async fn main(mut ctx: Context) -> Result<()> {
    let _udp = UdpTransport::create(&ctx).await?;
    let r = route![(UDP, "localhost:8000"), "echoer"];
    ctx.send(r, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
