use ockam::{Address, Context, LocalEntity, Result, Route, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Create a cloud node by going to https://hub.ockam.network
    let cloud_node_tcp_address = "40.78.99.34:4000"; //"Paste the tcp address of your cloud node here.";

    let secure_channel_listener_forwarding_address = "43537ada";
    //    "Paste the forwarding address of the secure channel here.";

    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection to your cloud node.
    tcp.connect(cloud_node_tcp_address).await?;

    let mut initiator = LocalEntity::create(&ctx, "initiator").await?;
    let cloud_node_address: Address = (TCP, cloud_node_tcp_address).into();
    let cloude_node_route: Route = vec![
        cloud_node_address,
        secure_channel_listener_forwarding_address.into(),
    ]
    .into();

    let channel = initiator.create_secure_channel(cloude_node_route).await?;

    let echoer_route: Route = vec![channel, "echoer".into()].into();

    ctx.send(echoer_route, "Hello world!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
