use crate::PortalMessage;
use async_trait::async_trait;
use ockam_core::compat::collections::VecDeque;
use ockam_core::{
    Address, Any, LocalMessage, Message, Result, Route, Routed, TransportMessage, Worker,
};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use std::net::SocketAddr;
use tokio::{io::AsyncWriteExt, net::tcp::OwnedWriteHalf};
use tracing::warn;

/// A TCP sending message worker
///
/// Create this worker type by calling
/// [`start_tcp_worker`](crate::start_tcp_worker)!
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for messages from the node message system
/// to dispatch to a remote peer.
pub(crate) struct TcpPortalSendWorker {
    tx: OwnedWriteHalf,
    peer: SocketAddr,
    internal_address: Address,
    remote_address: Address,
    onward_route: Option<Route>,
    buffer: VecDeque<Vec<u8>>,
}

impl TcpPortalSendWorker {
    pub fn new(
        tx: OwnedWriteHalf,
        peer: SocketAddr,
        internal_address: Address,
        remote_address: Address,
        onward_route: Option<Route>,
    ) -> Self {
        Self {
            tx,
            peer,
            internal_address,
            remote_address,
            onward_route,
            buffer: VecDeque::new(),
        }
    }
}

impl TcpPortalSendWorker {
    fn prepare_message(&mut self, msg: &Vec<u8>) -> Result<Vec<u8>> {
        let msg = PortalMessage::decode(&msg)?;

        Ok(msg.binary)
    }
}

#[async_trait]
impl Worker for TcpPortalSendWorker {
    type Context = Context;
    type Message = Any;

    // TcpSendWorker will receive messages from the TcpRouter to send
    // across the TcpStream to our friend
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        // Remove our own address from the route so the other end
        // knows what to do with the incoming message
        let mut onward_route = msg.onward_route();
        let recipient = onward_route.step()?;

        if onward_route.next().is_ok() {
            return Err(TransportError::UnknownRoute.into());
        }

        if recipient == self.internal_address {
            // Forward message
            let payload = msg.payload().clone();
            if let Some(r) = &self.onward_route {
                let msg = TransportMessage::v1(r.clone(), self.remote_address.clone(), payload);
                ctx.forward(LocalMessage::new(msg, Vec::new())).await?;
            } else {
                self.buffer.push_back(payload);
            }
        } else {
            let onward_route = msg.return_route();
            let onward_value = self.onward_route.take();
            if onward_value.is_none() {
                while let Some(msg) = self.buffer.pop_front() {
                    let msg = TransportMessage::v1(
                        onward_route.clone(),
                        self.remote_address.clone(),
                        msg,
                    );
                    ctx.forward(LocalMessage::new(msg, Vec::new())).await?;
                }
            }
            // Update route
            self.onward_route = Some(onward_route.clone());

            // Send to Tcp stream
            // Create a message buffer with pre-pended length
            let msg = self.prepare_message(msg.payload())?;

            if self.tx.write(&msg).await.is_err() {
                warn!("Failed to send message to peer {}", self.peer);
                ctx.stop_worker(ctx.address()).await?;
            }
        }

        Ok(())
    }
}
