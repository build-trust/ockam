use ockam_core::compat::rand::{self, Rng};
use ockam_core::{route, Result};
use ockam_node::workers::Echoer;
use ockam_node::Context;
use ockam_transport_tcp::{TcpConnectionOptions, TcpListenerOptions, TcpTransport};

#[ockam_macros::test]
async fn send_receive(ctx: &mut Context) -> Result<()> {
    let options = TcpListenerOptions::new();
    ctx.flow_controls()
        .add_consumer(&"echoer".into(), &options.spawner_flow_control_id());
    ctx.start_worker("echoer", Echoer)?;

    let transport = TcpTransport::create(ctx)?;
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
    Ok(())
}
