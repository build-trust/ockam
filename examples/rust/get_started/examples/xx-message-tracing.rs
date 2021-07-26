use ockam::{route, TCP, Context, RemoteTracer, Result, TcpTransport};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Create a cloud node by going to https://hub.ockam.network
    let cloud_node_tcp_address = "127.0.0.1:4000";

    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection to your cloud node.
    tcp.connect(cloud_node_tcp_address).await?;

    // Create a tracer to trace messages to the app address
    let tracer =
        RemoteTracer::create(&ctx, route![(TCP, cloud_node_tcp_address)], ctx.address()).await?;

    println!(
        "Use this address in the Hub route to trace messages: {}",
        tracer.remote_address()
    );

    let reply = ctx.receive_timeout::<String>(1000).await?;

    println!("Traced payload: {}", reply);

    ctx.stop().await
}
