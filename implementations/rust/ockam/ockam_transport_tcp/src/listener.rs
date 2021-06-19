use crate::{
    atomic::{self, ArcBool},
    TcpError, WorkerPair,
};
use async_trait::async_trait;
use ockam_core::{Address, Result, RouterMessage, Worker};
use ockam_node::Context;
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
        let waddr = Address::random(0);

        debug!("Binding TcpListener to {}", addr);
        let inner = TcpListener::bind(addr).await.map_err(TcpError::from)?;
        let worker = Self {
            inner,
            run,
            router_addr,
        };

        ctx.start_worker(waddr, worker).await?;
        Ok(())
    }
}

#[async_trait]
impl Worker for TcpListenWorker {
    type Context = Context;

    // Do not actually listen for messages
    type Message = ();

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        // FIXME: see ArcBool future note
        while atomic::check(&self.run) {
            trace!("Waiting for incoming TCP connection...");

            // Wait for an incoming connection
            let (stream, peer) = self.inner.accept().await.map_err(TcpError::from)?;

            // And spawn a connection worker for it
            let pair = WorkerPair::with_stream(ctx, stream, peer).await?;

            // Register the connection with the local TcpRouter
            ctx.send(
                self.router_addr.clone(),
                RouterMessage::Register {
                    accepts: format!("{}#{}", crate::TCP, peer).into(),
                    self_addr: pair.tx_addr.clone(),
                },
            )
            .await?;
        }

        ctx.stop_worker(ctx.address()).await
    }
}
