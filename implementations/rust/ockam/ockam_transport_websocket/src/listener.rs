use std::net::SocketAddr;

use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::StreamExt;
use ockam_core::{async_trait, Address, Result, RouterMessage, Worker};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use tokio::net::TcpListener;

use crate::atomic::{self, ArcBool};
use crate::init::WorkerPair;
use crate::WebSocketError;

pub struct WebSocketListenWorker {
    inner: TcpListener,
    run: ArcBool,
    router_addr: Address,
}

impl WebSocketListenWorker {
    pub(crate) async fn start(
        ctx: &Context,
        router_addr: Address,
        addr: SocketAddr,
        run: ArcBool,
    ) -> Result<()> {
        debug!("Binding WebSocketListener to {}", addr);
        let inner = TcpListener::bind(addr)
            .await
            .map_err(TransportError::from)?;
        let worker = Self {
            inner,
            run,
            router_addr,
        };
        let waddr = Address::random(0);
        ctx.start_worker(waddr, worker).await?;
        Ok(())
    }

    async fn accept_tcp_streams(
        &self,
        ctx: &Context,
        tx: UnboundedSender<WorkerPair>,
    ) -> Result<()> {
        while let Ok((tcp_stream, peer)) = self.inner.accept().await {
            if !atomic::check(&self.run) {
                debug!("WebSocketListenWorker stopped");
                tx.close_channel();
                break;
            }
            if tx.is_closed() {
                debug!("WorkerPair tx closed");
                break;
            }

            debug!("TcpStream accepted");
            let stream = tokio_tungstenite::accept_async(tcp_stream)
                .await
                .map_err(WebSocketError::from)?;
            let pair = WorkerPair::with_stream(ctx, stream, peer).await?;
            let try_send_pair = tx.unbounded_send(pair).map_err(WebSocketError::from);
            if let Err(err) = try_send_pair {
                warn!(
                    "Failed to send WorkerPair through channel. {}",
                    err.to_string()
                );
            }
        }
        trace!("Exiting accept_tcp_streams");
        Ok(())
    }

    async fn on_tcp_stream_accepted(
        &self,
        ctx: &Context,
        mut rx: UnboundedReceiver<WorkerPair>,
    ) -> Result<()> {
        while let Some(pair) = rx.next().await {
            // Register the connection with the local WebSocketRouter
            trace!("Sending register message...");
            ctx.send(
                self.router_addr.clone(),
                RouterMessage::Register {
                    accepts: vec![format!("{}#{}", crate::WS, pair.peer).into()],
                    self_addr: pair.tx_addr.clone(),
                },
            )
            .await?;
        }
        trace!("Exiting on_tcp_stream_accepted");
        Ok(())
    }
}

#[async_trait::async_trait]
impl Worker for WebSocketListenWorker {
    // Do not actually listen for messages
    type Message = ();
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        trace!("Waiting for incoming TCP connection...");
        let (tx, rx) = futures_channel::mpsc::unbounded();
        let (handled_accept_tcp_streams, handled_on_tcp_stream_accepted) = tokio::join!(
            self.accept_tcp_streams(ctx, tx),
            self.on_tcp_stream_accepted(ctx, rx)
        );
        ctx.stop_worker(ctx.address()).await?;
        handled_accept_tcp_streams?;
        handled_on_tcp_stream_accepted?;
        Ok(())
    }
}
