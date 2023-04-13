use ockam_core::compat::rand::{self, Rng};
use ockam_core::flow_control::{FlowControlPolicy, FlowControls};
use ockam_core::{route, AllowAll, Result, Routed, Worker};
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
    let listener_flow_control_id = FlowControls::generate_id();
    ctx.flow_controls().add_consumer(
        "echoer",
        &listener_flow_control_id,
        FlowControlPolicy::SpawnerAllowMultipleMessages,
    );
    ctx.start_worker("echoer", Echoer, AllowAll, AllowAll)
        .await?;

    let transport = TcpTransport::create(ctx).await?;
    let (listener_address, _) = transport
        .listen(
            "127.0.0.1:0",
            TcpListenerOptions::new(&listener_flow_control_id),
        )
        .await?;

    let addr = transport
        .connect(listener_address.to_string(), TcpConnectionOptions::new())
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
