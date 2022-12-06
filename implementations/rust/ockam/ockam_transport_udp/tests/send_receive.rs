use ockam_core::compat::rand::{self, Rng};
use ockam_core::{route, AllowAll, Result, Routed, Worker};
use ockam_node::Context;
use std::sync::Arc;

use ockam_transport_udp::{UdpTransport, UDP};
use tracing::debug;

#[ockam_macros::test]
async fn send_receive(ctx: &mut Context) -> Result<()> {
    let rand_port = rand::thread_rng().gen_range(10000..65535);
    let bind_address = format!("127.0.0.1:{}", rand_port);
    let bind_address = bind_address.as_str();

    // Listener
    {
        let transport = UdpTransport::create(ctx).await?;
        transport.listen(bind_address).await?;
        ctx.start_worker("echoer", Echoer, Arc::new(AllowAll), Arc::new(AllowAll))
            .await?;
    };

    // Sender
    {
        let msg: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(256)
            .map(char::from)
            .collect();
        let r = route![(UDP, bind_address), "echoer"];
        let reply: String = ctx.send_and_receive(r, msg.clone()).await?;

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
        debug!("Replying back to {}", &msg.return_route());
        ctx.send(msg.return_route(), msg.body()).await
    }
}
