use ockam::{
    route, Context, Entity, Result, SecureChannels, TcpTransport, TrustEveryonePolicy, Vault, TCP,
};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Using a shared Hub Node.
    // You can create a personal node by going to https://hub.ockam.network
    let hub_node_tcp_address = "54.151.52.111:4000";

    let forwarding_address = "<Paste the forwarding address of Bob here>";

    // Initialize the TCP Transport.
    let _tcp = TcpTransport::create(&ctx).await?;

    let vault = Vault::create(&ctx).expect("failed to create vault");
    let mut alice = Entity::create(&ctx, &vault)?;

    let hub_node_route = route![(TCP, hub_node_tcp_address), forwarding_address];
    let channel = alice.create_secure_channel(hub_node_route, TrustEveryonePolicy)?;

    ctx.send(route![channel, "app"], "Hello Ockam!".to_string())
        .await?;

    Ok(())
}
