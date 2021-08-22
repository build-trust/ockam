use core::iter;
use std::time::Duration;

use ockam_core::compat::sync::Arc;
use ockam_core::{route, Error, Result, Routed, Worker};
use ockam_node::Context;
use rand::Rng;
use tokio::sync::{oneshot, RwLock};
use tokio::time::timeout;

use ockam_transport_tcp::{TcpTransport, TCP};

#[tokio::test(flavor = "multi_thread")]
async fn send_receive() {
    let (tx, rx) = oneshot::channel::<Result<()>>();
    let (ctx, mut executor) = ockam_node::start_node();
    executor
        .execute(async move { run_with_timeout(tx, ctx).await })
        .expect("Executor should not fail");
    assert!(rx.await.unwrap().is_ok(), "'run' method should return Ok");
}

async fn run_with_timeout(tx: oneshot::Sender<Result<()>>, ctx: Context) {
    let ctx = Arc::new(RwLock::new(ctx));
    match timeout(Duration::from_secs(1), run_test(ctx.clone())).await {
        Ok(res) => tx.send(res).unwrap(),
        Err(_) => tx.send(Err(Error::new(0, ""))).unwrap(),
    };
    let mut ctx = ctx.write().await;
    let _ = ctx.stop().await;
}

async fn run_test(ctx: Arc<RwLock<Context>>) -> Result<()> {
    let mut ctx = ctx.write().await;
    let transport = TcpTransport::create(&ctx).await?;
    transport.listen("127.0.0.1:4000").await?;

    ctx.start_worker("echoer", Echoer).await?;

    let msg: String = {
        let mut rng = rand::thread_rng();
        iter::repeat(())
            .map(|()| rng.sample(&rand::distributions::Alphanumeric))
            .take(10)
            .collect()
    };
    let r = route![(TCP, "127.0.0.1:4000"), "echoer"];
    ctx.send(r, msg.clone()).await?;

    let reply = ctx.receive::<String>().await?;
    assert_eq!(reply, msg, "Should receive the same message");
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
