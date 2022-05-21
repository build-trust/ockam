use ockam_core::compat::rand::{self, Rng};
use ockam_core::{route, Address, Result, Routed, Worker};
use ockam_node::Context;

use ockam_transport_tcp::{TcpTransport, TCP};

#[ockam_macros::test]
async fn send_receive(ctx: &mut Context) -> Result<()> {
    let transport = TcpTransport::create(ctx).await?;
    let listener_address = transport.listen("127.0.0.1:0").await?;
    ctx.start_worker("echoer", Echoer).await?;

    let _sender = {
        let msg: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(256)
            .map(char::from)
            .collect();

        let r = route![(TCP, listener_address.to_string()), "echoer"];

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
async fn tcp_lifecycle__reconnect__should_not_error(ctx: &mut Context) -> Result<()> {
    ctx.start_worker("echoer", Echoer).await?;

    let transport = TcpTransport::create(ctx).await?;
    let listener_address = transport.listen("127.0.0.1:0").await?.to_string();

    let mut child_ctx = ctx.new_context(Address::random_local()).await?;
    let msg: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(256)
        .map(char::from)
        .collect();

    let tx_address = transport.connect(&listener_address).await?;

    let r = route![(TCP, listener_address.clone()), "echoer"];
    child_ctx.send(r.clone(), msg.clone()).await?;

    let reply = child_ctx.receive::<String>().await?;
    assert_eq!(reply, msg, "Should receive the same message");

    transport.disconnect(&listener_address).await?;

    // TcpSender address should not exist
    let res = child_ctx.send(tx_address.clone(), "TEST".to_string()).await;
    assert!(res.is_err());

    // FIXME!
    // assert_eq!(
    //     res.err().unwrap(),
    //     ockam_node::error::Error::UnknownAddress.into()
    // );

    // This should create new connection
    child_ctx
        .send(route![(TCP, listener_address), "echoer"], msg.clone())
        .await?;

    let reply = child_ctx.receive::<String>().await?;
    assert_eq!(reply, msg, "Should receive the same message");

    if let Err(e) = ctx.stop().await {
        println!("Unclean stop: {}", e)
    }

    Ok(())
}
