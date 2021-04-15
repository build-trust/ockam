#[macro_use]
extern crate tracing;

use ockam::{Context, Result, Route};
use ockam_transport_tcp::TcpTransport;
use std::io;

fn get_peer_addr() -> String {
    std::env::args()
        .skip(1)
        .take(1)
        .next()
        // This value can be used when running the ockam-hub locally
        .unwrap_or(format!("127.0.0.1:4000"))
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Get our peer address
    let peer = get_peer_addr();

    // Initialize the TCP stack by opening a connection to a the remote
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(peer.clone()).await?;

    // Get the forwarding route from user input
    let mut buffer = String::new();
    println!("Paste the forwarding route below ↓");
    io::stdin().read_line(&mut buffer).unwrap();
    let route = Route::parse(buffer).unwrap_or_else(|| {
        error!("Failed to parse route!");
        eprintln!("Route format [type#]<address> [=> [type#]<address>]+");
        std::process::exit(1);
    });

    debug!("Sending message to route: {}", route);

    ctx.send(route, String::from("Hello you!")).await?;

    // We can't shut down the node here because otherwise a race
    // condition will drop the tcp messages in transit.
    Ok(())
}
