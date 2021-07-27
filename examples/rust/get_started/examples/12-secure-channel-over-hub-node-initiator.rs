use ockam::{
    route, Context, Entity, Result, SecureChannels, TcpTransport, TrustEveryonePolicy, Vault, TCP,
};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Create a hub node by going to https://hub.ockam.network
    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>"; // e.g. "127.0.0.1:4000"

    let forwarding_address = "<Address copied from responder output>";

    // Initialize the TCP Transport.
    let _tcp = TcpTransport::create(&ctx).await?;

    let vault = Vault::create(&ctx).expect("failed to create vault");
    let mut alice = Entity::create(&ctx, &vault)?;

    let hub_node_route = route![(TCP, hub_node_tcp_address), forwarding_address];

    let channel = alice.create_secure_channel(hub_node_route, TrustEveryonePolicy)?;

    let echoer_route = route![channel, "echoer"];

    ctx.send(echoer_route, "Hello world!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
