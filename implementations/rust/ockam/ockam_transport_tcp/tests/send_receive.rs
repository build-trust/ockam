use ockam_core::compat::rand::{self, Rng};
use ockam_core::compat::sync::Arc;
use ockam_core::{route, AllowAll, Mailboxes, Result, Routed, Worker};
use ockam_node::{Context, WorkerBuilder};

use ockam_transport_tcp::TcpTransport;
use std::time::Duration;
use tracing::info;

#[ockam_macros::test]
async fn send_receive(ctx: &mut Context) -> Result<()> {
    let transport = TcpTransport::create(ctx).await?;
    let listener_address = transport.listen("127.0.0.1:0").await?;
    WorkerBuilder::with_mailboxes(
        Mailboxes::main("echoer", Arc::new(AllowAll), Arc::new(AllowAll)),
        Echoer,
    )
    .start(ctx)
    .await?;

    let addr = transport.connect(listener_address.to_string()).await?;

    // Sender
    {
        let msg: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(256)
            .map(char::from)
            .collect();

        let r = route![addr, "echoer"];

        let reply = ctx.send_and_receive::<_, _, String>(r, msg.clone()).await?;

        assert_eq!(reply, msg, "Should receive the same message");
    };

    if let Err(e) = ctx.stop().await {
        println!("Unclean stop: {}", e)
    }

    Ok(())
}

pub struct Echoer;

#[ockam_core::worker]
impl Worker for Echoer {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        ctx.send(msg.return_route(), msg.body()).await
    }
}

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn tcp_lifecycle__two_connections__should_both_work(ctx: &mut Context) -> Result<()> {
    ctx.start_worker("echoer", Echoer, AllowAll, AllowAll)
        .await?;

    let transport = TcpTransport::create(ctx).await?;
    let listener_address = transport.listen("127.0.0.1:0").await?.to_string();

    let msg1: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(256)
        .map(char::from)
        .collect();
    let msg2: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(256)
        .map(char::from)
        .collect();

    let tx_address1 = transport.connect(&listener_address).await?;

    let reply1: String = ctx
        .send_and_receive(route![tx_address1.clone(), "echoer"], msg1.clone())
        .await?;
    assert_eq!(reply1, msg1, "Should receive the same message");

    let tx_address2 = transport.connect(&listener_address).await?;
    let reply2: String = ctx
        .send_and_receive(route![tx_address2.clone(), "echoer"], msg2.clone())
        .await?;
    assert_eq!(reply2, msg2, "Should receive the same message");

    if let Err(e) = ctx.stop().await {
        println!("Unclean stop: {}", e)
    }

    Ok(())
}

#[ignore]
#[ockam_macros::test(timeout = 400000)]
async fn tcp_keepalive_test(ctx: &mut Context) -> Result<()> {
    let tcp = TcpTransport::create(ctx).await?;

    let message: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(256)
        .map(char::from)
        .collect();

    let cloud = tcp.connect("1.node.ockam.network:4000").await?;

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

    if let Err(e) = ctx.stop().await {
        println!("Unclean stop: {}", e)
    }

    Ok(())
}
