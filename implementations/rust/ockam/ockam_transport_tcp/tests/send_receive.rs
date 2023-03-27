use ockam_core::compat::rand::{self, Rng};
use ockam_core::compat::sync::Arc;
use ockam_core::{route, AllowAll, Mailboxes, Result, Routed, Worker};
use ockam_node::{Context, WorkerBuilder};
use ockam_transport_tcp::{TcpConnectionTrustOptions, TcpListenerTrustOptions, TcpTransport};

pub struct Echoer;

#[ockam_core::worker]
impl Worker for Echoer {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        ctx.send(msg.return_route(), msg.body()).await
    }
}

#[ockam_macros::test]
async fn send_receive(ctx: &mut Context) -> Result<()> {
    let transport = TcpTransport::create(ctx).await?;
    let (listener_address, _) = transport
        .listen("127.0.0.1:0", TcpListenerTrustOptions::insecure_test())
        .await?;
    WorkerBuilder::with_mailboxes(
        Mailboxes::main("echoer", Arc::new(AllowAll), Arc::new(AllowAll)),
        Echoer,
    )
    .start(ctx)
    .await?;

    let addr = transport
        .connect(
            listener_address.to_string(),
            TcpConnectionTrustOptions::insecure_test(),
        )
        .await?;

    // Sender
    {
        let msg: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(256)
            .map(char::from)
            .collect();

        let r = route![addr, "echoer"];

        let reply = ctx.send_and_receive::<String>(r, msg.clone()).await?;

        assert_eq!(reply, msg, "Should receive the same message");
    };

    if let Err(e) = ctx.stop().await {
        println!("Unclean stop: {}", e)
    }

    Ok(())
}
