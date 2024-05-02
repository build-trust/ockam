use ockam::route;
use ockam_core::Result;
use ockam_node::Context;
use ockam_transport_udp::{UdpBindArguments, UdpBindOptions, UdpTransport, UDP};

#[ockam_macros::node]
async fn main(ctx: Context) -> Result<()> {
    let udp = UdpTransport::create(&ctx).await?;

    let bind = udp
        .bind(UdpBindArguments::new(), UdpBindOptions::new())
        .await?;

    let r = route![bind, (UDP, "localhost:8000"), "echoer"];

    // Wait to receive a reply and print it.
    let reply: String = ctx.send_and_receive(r, "Hello Ockam!".to_string()).await?;

    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
