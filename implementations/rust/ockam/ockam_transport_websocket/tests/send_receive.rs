use ockam_core::compat::rand::{self, Rng};
use ockam_core::{route, Address, Result, Routed, Worker};
use ockam_node::Context;
use ockam_transport_websocket::{WebSocketTransport, WS};

#[ockam_macros::test]
async fn send_receive(ctx: &mut Context) -> Result<()> {
    let listener_address;

    let _listener = {
        let transport = WebSocketTransport::create(ctx).await?;
        listener_address = transport.listen("127.0.0.1:0").await?;
        ctx.start_worker("echoer", Echoer).await?;
    };

    let _sender = {
        let mut ctx = ctx.new_context(Address::random_local()).await?;
        let msg: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(256)
            .map(char::from)
            .collect();
        let r = route![(WS, listener_address.to_string()), "echoer"];
        ctx.send(r, msg.clone()).await?;

        let reply = ctx.receive::<String>().await?;
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
