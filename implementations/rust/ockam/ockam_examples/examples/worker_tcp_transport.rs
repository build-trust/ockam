use ockam::{Context, Result, Route};
use ockam_transport_tcp::{self as tcp, TcpRouter};
use std::net::SocketAddr;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Create and register the TCP router
    let router_handle = TcpRouter::register(&ctx).await?;

    // Create and register a connection to ockam hub
    let addr: SocketAddr = "138.91.152.195:4000".parse().unwrap();
    let connection = tcp::start_tcp_worker(&ctx, addr).await?;
    router_handle.register(&connection).await?;

    // Create a route to the remote `echo_service`
    let route = Route::new()
        .append_t(1, "138.91.152.195:4000")
        .append("echo_service");

    // Send the message
    let msg = "Hello you over there!".to_string();
    println!("Sending message: '{}'", msg);
    ctx.send_message(route, msg).await?;

    // Wait for and print the reply
    let reply = ctx.receive::<String>().await?;
    println!("Echo service says: '{}'", reply);

    // Shut down the node
    ctx.stop().await
}
