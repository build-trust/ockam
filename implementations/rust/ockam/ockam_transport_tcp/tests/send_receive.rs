use ockam_core::compat::rand::{self, Rng};
use ockam_core::{route, Result, Routed, Worker};
use ockam_node::Context;
use ockam_transport_tcp::{TcpConnectionOptions, TcpListenerOptions, TcpTransport};

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
    let options = TcpListenerOptions::new();
    ctx.flow_controls()
        .add_consumer("echoer", &options.spawner_flow_control_id());
    ctx.start_worker("echoer", Echoer).await?;

    let transport = TcpTransport::create(ctx).await?;
    let listener = transport.listen("127.0.0.1:0", options).await?;

    let addr = transport
        .connect(listener.socket_string(), TcpConnectionOptions::new())
        .await?
        .sender_address()
        .clone();

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
