use futures_channel::mpsc::{channel, Receiver, Sender};
use futures_util::StreamExt;
use ockam_core::lib::net::SocketAddr;
use ockam_core::{async_trait, worker, Address, Result, RouterMessage, Worker};
use ockam_node::Context;
use tokio::net::TcpListener;
use tracing::{debug, trace, warn};

use crate::common::{ConnectionListenerWorker, TransportError, TransportNode};
use crate::node::TransportNodeWebSocket;

pub(crate) struct ConnectionListenerWorkerWebSocket {
    inner: TcpListener,
    addr: Address,
    router: Address,
}

impl ConnectionListenerWorkerWebSocket {
    async fn accept_streams(
        &self,
        ctx: &Context,
        mut tx: Sender<TransportNodeWebSocket>,
    ) -> Result<()> {
        while let Ok((tcp_stream, peer)) = self.inner.accept().await {
            if tx.is_closed() {
                debug!("ConnectionListenerWorker tx channel closed");
                break;
            }

            trace!("TcpStream received");
            let stream = tokio_tungstenite::accept_async(tcp_stream)
                .await
                .map_err(TransportError::from)?;
            let node = TransportNodeWebSocket::new(peer);
            node.start(ctx, stream).await?;
            if let Err(err) = tx.try_send(node).map_err(TransportError::from) {
                warn!(
                    "Failed to send WorkerPair through channel. {}",
                    err.to_string()
                );
            }
        }
        trace!("Exiting accept_streams");
        Ok(())
    }

    async fn on_stream_accepted(
        &self,
        ctx: &Context,
        mut rx: Receiver<TransportNodeWebSocket>,
    ) -> Result<()> {
        while let Some(node) = rx.next().await {
            trace!("Sending register message to local Router...");
            ctx.send(
                self.router.clone(),
                RouterMessage::Register {
                    accepts: vec![TransportNodeWebSocket::build_addr(node.peer())],
                    self_addr: node.tx_addr(),
                },
            )
            .await?;
        }
        trace!("Exiting on_stream_accepted");
        Ok(())
    }
}

#[async_trait::async_trait]
impl ConnectionListenerWorker for ConnectionListenerWorkerWebSocket {
    type Transport = TransportNodeWebSocket;

    async fn start(ctx: &Context, addr: SocketAddr, router: Address) -> Result<()> {
        debug!("Binding WebSocketListener to {}", addr);
        let inner = TcpListener::bind(addr)
            .await
            .map_err(TransportError::from)?;
        let worker = Self {
            inner,
            addr: Address::random(0),
            router,
        };
        ctx.start_worker(worker.addr.clone(), worker).await
    }
}

#[worker]
impl Worker for ConnectionListenerWorkerWebSocket {
    type Message = ();
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        trace!("Waiting for incoming TCP connections...");
        let (tx, rx) = channel(32);
        let (handle_accept_streams, handle_on_stream_accepted) = tokio::join!(
            self.accept_streams(ctx, tx),
            self.on_stream_accepted(ctx, rx)
        );
        trace!("Dropping connection listener");
        ctx.stop_worker(self.addr.clone()).await?;
        handle_accept_streams?;
        handle_on_stream_accepted?;
        Ok(())
    }
}
