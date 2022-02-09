#[macro_use]
extern crate tracing;

use ockam::{Context, Result, Route};
use ockam_transport_tcp::{TcpTransport, TCP};

fn get_peer_addr() -> String {
    std::env::args()
        .skip(1)
        .take(1)
        .next()
        .unwrap_or(format!("127.0.0.1:10222"))
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Get our peer address
    let peer_addr = get_peer_addr();

    // Initialize the TCP stack by opening a connection to a the remote
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(&peer_addr).await?;

    // Send a message to the remote
    ctx.send(
        Route::new()
            .append_t(TCP, &peer_addr)
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
