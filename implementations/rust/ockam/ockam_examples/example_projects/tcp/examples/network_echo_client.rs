#[macro_use]
extern crate tracing;

use ockam::{Context, Result, Route};
use ockam_transport_tcp::TcpTransport;
use std::net::SocketAddr;

fn get_peer_addr() -> SocketAddr {
    std::env::args()
        .skip(1)
        .take(1)
        .next()
        .unwrap_or(format!("127.0.0.1:10222"))
        .parse()
        .ok()
        .unwrap_or_else(|| {
            error!("Failed to parse socket address!");
            eprintln!("Usage: network_echo_client <ip>:<port>");
            std::process::exit(1);
        })
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Get our peer address
    let peer_addr = get_peer_addr();

    // Initialize the TCP stack by opening a connection to a the remote
    TcpTransport::create(&ctx, peer_addr).await.unwrap();

    // Send a message to the remote
    ctx.send(
        Route::new()
            .append(format!("1#{}", peer_addr))
            .append("echo_service"),
        String::from("Hello you over there!"),
    )
    .await?;

    // Then wait for a message back!
    let msg = ctx.receive::<String>().await?;
    info!("Received return message: '{}'", msg);

    ctx.stop().await?;
    Ok(())
}
