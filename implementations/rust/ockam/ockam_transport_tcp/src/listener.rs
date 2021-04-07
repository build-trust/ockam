use crate::{
    atomic::{self, ArcBool},
    WorkerPair,
};
use ockam::{async_worker, Address, Context, Result, RouterMessage, Worker};
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub struct TcpListenWorker {
    inner: TcpListener,
    run: ArcBool,
    router_addr: Address,
}

impl TcpListenWorker {
    pub(crate) async fn start(
        ctx: &Context,
        router_addr: Address,
        addr: SocketAddr,
        run: ArcBool,
    ) -> Result<()> {
        let waddr = format!("{}_listener", addr);

        debug!("Binding TcpListener to {}", addr);
        let inner = TcpListener::bind(addr).await.unwrap();
        let worker = Self {
            inner,
            run,
            router_addr,
        };

        ctx.start_worker(waddr.as_str(), worker).await?;
        Ok(())
    }
}

#[async_worker]
impl Worker for TcpListenWorker {
    type Context = Context;

    // Do not actually listen for messages
    type Message = ();

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        // FIXME: see ArcBool future note
        while atomic::check(&self.run) {
            trace!("Waiting for incoming TCP connection...");

            // Wait for an incoming connection
            let (stream, peer) = self.inner.accept().await.unwrap();

            // And spawn a connection worker for it
            let pair = WorkerPair::with_stream(ctx, stream, peer).await?;

            // Register the connection with the local TcpRouter
            ctx.send_message(
                self.router_addr.clone(),
                RouterMessage::Register {
                    accepts: format!("1#{}", peer).into(),
                    self_addr: pair.tx_addr.clone(),
                },
            )
            .await?;
        }

        ctx.stop_worker(ctx.primary_address()).await
    }
}
