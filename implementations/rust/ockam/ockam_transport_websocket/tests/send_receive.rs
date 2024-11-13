use ockam_core::compat::rand::{self, Rng};
use ockam_core::{route, Result};
use ockam_node::workers::Echoer;
use ockam_node::Context;
use ockam_transport_websocket::{WebSocketTransport, WS};

#[ignore]
#[ockam_macros::test]
async fn send_receive(ctx: &mut Context) -> Result<()> {
    let transport = WebSocketTransport::create(ctx)?;
    let listener_address = transport.listen("127.0.0.1:0").await?;
    ctx.start_worker("echoer", Echoer)?;

    // Sender
    {
        let msg: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(256)
            .map(char::from)
            .collect();
        let r = route![(WS, listener_address.to_string()), "echoer"];
        let reply = ctx.send_and_receive::<String>(r, msg.clone()).await?;

        assert_eq!(reply, msg, "Should receive the same message");
    };
    Ok(())
}
