use ockam::{route, Context, RemoteForwarder, Result, TcpTransport, TCP};
use ockam_get_started::Echoer;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a hub node by going to https://hub.ockam.network
    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>"; // e.g. "127.0.0.1:4000"

    // Initialize the TCP Transport.
    let _tcp = TcpTransport::create(&ctx).await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer).await?;

    // Ask the forwarding_service, in your node in Ockam Hub,
    // to create a forwarder that would forward all its messages
    // to the "echoer" worker we created above.
    let forwarder =
        RemoteForwarder::create(&ctx, route![(TCP, hub_node_tcp_address)], "echoer").await?;

    println!("Forwarding address: {}", forwarder.remote_address());

    Ok(())
}
