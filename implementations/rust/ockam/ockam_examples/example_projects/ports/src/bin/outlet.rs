use ockam::{route, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // This is local address at which we'll start outlet
    let outlet_address = "outlet";

    // Create outlet that will stream messages to localhost port 22 (sshd)
    tcp.create_outlet(outlet_address, "localhost:22").await?;

    // Connect to the other node, and tell what is outlet address
    ctx.send(
        // The other node is listening on localhost port 1234
        // "app" is the address of apps main function Context
        route![(TCP, "localhost:1234"), "app"],
        outlet_address.to_string(),
    )
    .await?;

    // We won't call ctx.stop() here, this program will run until you stop it with Ctrl-C
    Ok(())
}
