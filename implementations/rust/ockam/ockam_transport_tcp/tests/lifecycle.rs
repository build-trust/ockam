use ockam_core::compat::rand::{self, Rng};
use ockam_core::{route, AllowAll, Result, Routed, Worker};
use ockam_node::Context;
use ockam_transport_tcp::TcpTransport;

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
    let listener_address = transport.listen("127.0.0.1:0").await?.0.to_string();

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
