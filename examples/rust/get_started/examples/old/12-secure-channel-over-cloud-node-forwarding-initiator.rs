use ockam::{
    route, Context, Entity, Result, TcpTransport, TrustEveryonePolicy, Vault, TCP,
};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Create a cloud node by going to https://hub.ockam.network
    let cloud_node_tcp_address = "Paste the tcp address of your cloud node here.";

    let secure_channel_listener_forwarding_address =
        "Paste the forwarding address of the secure channel here.";

    // Initialize the TCP Transport.
    let _tcp = TcpTransport::create(&ctx).await?;

    let vault = Vault::create(&ctx).expect("failed to create vault");
    let mut alice = Entity::create(&ctx, &vault).await?;
    let cloud_node_route = route![
        (TCP, cloud_node_tcp_address),
        secure_channel_listener_forwarding_address
    ];

    let channel = alice.create_secure_channel(cloud_node_route, TrustEveryonePolicy).await?;

    let echoer_route = route![channel, "echoer"];

    ctx.send(echoer_route, "Hello world!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
