use ockam_core::compat::rand::{self, Rng};
use ockam_core::{route, Result};
use ockam_node::Context;
use ockam_transport_tcp::{TcpConnectionOptions, TcpTransport};
use std::time::Duration;
use tracing::info;

#[ignore]
#[ockam_macros::test(timeout = 400000)]
async fn tcp_keepalive_test(ctx: &mut Context) -> Result<()> {
    let tcp = TcpTransport::create(ctx)?;

    let message: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(256)
        .map(char::from)
        .collect();

    let cloud = tcp
        .connect("1.node.ockam.network:4000", TcpConnectionOptions::new())
        .await?;

    // Send the message to the cloud node echoer
    // Wait to receive an echo and print it.
    let reply: String = ctx
        .send_and_receive(route![cloud.clone(), "echo"], message.to_string())
        .await?;
    info!("Sender has received the following echo: {}\n", reply);

    // Sleep the thread to allow the tcp socket to send keepalive probes
    let sleep_duration = Duration::from_secs(350);
    info!("Sleeping task now for {:?}", sleep_duration);
    ctx.sleep(sleep_duration).await;

    // Resend the message to the cloud node echoer to check if connection is still alive
    // Wait to receive an echo and print it.
    let reply: String = ctx
        .send_and_receive(route![cloud, "echo"], message.to_string())
        .await?;
    info!(
        "Sender has received the following echo after sleeping for {:?}: {}\n",
        sleep_duration, reply
    );

    if let Err(e) = ctx.shutdown_node().await {
        println!("Unclean stop: {}", e)
    }

    Ok(())
}
