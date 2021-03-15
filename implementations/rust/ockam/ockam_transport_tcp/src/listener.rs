use crate::{
    atomic::{self, ArcBool},
    WorkerPair,
};
use ockam::{async_worker, Context, Result, Worker};
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub struct TcpListenWorker {
    inner: TcpListener,
    run: ArcBool,
}

impl TcpListenWorker {
    pub(crate) async fn start(ctx: &Context, addr: SocketAddr, run: ArcBool) -> Result<()> {
        let waddr = format!("{}_listener", addr);
        let inner = TcpListener::bind(addr).await.unwrap();
        let worker = Self { inner, run };

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
            // Wait for an incoming connection
            let (stream, peer) = self.inner.accept().await.unwrap();

            // And spawn a connection worker for it
            WorkerPair::with_stream(ctx, stream, peer).await.unwrap();
        }

        ctx.stop_worker(ctx.address()).await
    }
}
