use core::iter;

use ockam_core::{route, Address, Result, Routed, Worker};
use ockam_node::Context;
use rand::Rng;

use ockam_transport_tcp::{TcpTransport, TCP};

#[ockam_macros::test]
async fn send_receive(ctx: &mut Context) -> Result<()> {
    let rand_port = rand::thread_rng().gen_range(10000, 65535);
    let bind_address = format!("127.0.0.1:{}", rand_port);
    let bind_address = bind_address.as_str();

    let _listener = {
        let transport = TcpTransport::create(ctx).await?;
        transport.listen(bind_address).await?;
        ctx.start_worker("echoer", Echoer).await?;
    };

    let _sender = {
        let mut ctx = ctx.new_context(Address::random(0)).await?;
        let msg: String = {
            let mut rng = rand::thread_rng();
            iter::repeat(())
                .map(|()| rng.sample(&rand::distributions::Alphanumeric))
                .take(10)
                .collect()
        };
        let r = route![(TCP, bind_address), "echoer"];
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
