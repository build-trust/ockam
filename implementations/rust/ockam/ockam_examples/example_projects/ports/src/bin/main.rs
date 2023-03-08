use ockam::{route, Context, Identity, Result, TcpTransport, TrustEveryonePolicy, Vault, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Create a Vault to safely store secret keys
    let vault = Vault::create(&ctx).await?;

    // Create an Identity to represent this machine
    let mut fabric_machine = Identities::create(&ctx, vault)?;

    // Initialize the TCP Transport
    let tcp = TcpTransport::create(&ctx).await?;

    // Hostname for your personal Ockam Hub Node
    let ockam_hub_hostname = "OCKAM_HUB_NODE_HOSTNAME:4000";

    // Create Secure Channel to your personal Ockam Hub Node
    let channel = fabric_machine.create_secure_channel(
        route![(TCP, ockam_hub_hostname), "secure_channel_listener_service"],
        TrustEveryonePolicy,
    )?;

    // This is local address at which we'll start outlet
    let outlet_address = "outlet";

    // Create outlet that will stream messages to localhost port 22 (sshd)
    tcp.create_outlet(outlet_address, "localhost:22").await?;

    // Ask Ockam Hub Node to create an Inlet for us, that will stream messages to our Outlet
    ctx.send(
        route![channel, "tcp_inlet_service"],
        outlet_address.to_string(),
    )
    .await?;

    // Ockam Hub responds with a port used for Inlet
    let port = ctx.receive::<i32>().await?.take().body();

    println!("Inlet is accessible on port {}", port);

    // We won't call ctx.stop() here, this program will run until you stop it with Ctrl-C
    Ok(())
}
