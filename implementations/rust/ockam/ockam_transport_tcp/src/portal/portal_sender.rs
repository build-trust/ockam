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

pub(crate) enum TcpPortalSendWorkerState {
    Inlet { listener_route: Route },
    Outlet,
}

/// A TCP sending message worker
///
/// Create this worker type by calling
/// [`start_tcp_worker`](crate::start_tcp_worker)!
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for messages from the node message system
/// to dispatch to a remote peer.
pub(crate) struct TcpPortalSendWorker {
    state: TcpPortalSendWorkerState,
    tx: OwnedWriteHalf,
    peer: SocketAddr,
    internal_address: Address,
    remote_address: Address,
    onward_route: Option<Route>,
    buffer: VecDeque<Vec<u8>>,
}

impl TcpPortalSendWorker {
    pub fn new(
        state: TcpPortalSendWorkerState,
        tx: OwnedWriteHalf,
        peer: SocketAddr,
        internal_address: Address,
        remote_address: Address,
    ) -> Self {
        Self {
            state,
            tx,
            peer,
            internal_address,
            remote_address,
            onward_route: None,
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

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        match &self.state {
            TcpPortalSendWorkerState::Inlet { listener_route } => {
                let empty_payload: Vec<u8> = vec![];
                // Force creation of Outlet on the other side
                ctx.send_from_address(
                    listener_route.clone(),
                    empty_payload,
                    self.remote_address.clone(),
                )
                .await?;
            }
            TcpPortalSendWorkerState::Outlet => {}
        }

        Ok(())
    }

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
            if self.onward_route.is_none() {
                // TODO: This should be serialized empty vec
                if msg.payload().len() != 1 {
                    return Err(TransportError::Protocol.into());
                }
                // Update route
                let onward_route = msg.return_route();
                self.onward_route = Some(onward_route.clone());

                // If onward route was none, there is two possible reasons for that:
                //    1. It's inlet, we sent initial empty message to outlet listener,
                //       it created new dedicated outlet worker for us, and this is empty
                //       response, the only purpose of which, is to give us correct route for
                //       further messages
                //    2. It's outlet, and it's empty initial message sent from inlet to inlet
                //       listener, that listener forwarded to us. We need to respond so that
                //       outlet is aware of route to us
                // Either way, don't stream this message to tcp stream
                match &self.state {
                    TcpPortalSendWorkerState::Inlet { .. } => {}
                    TcpPortalSendWorkerState::Outlet => {
                        let empty_payload: Vec<u8> = vec![];
                        ctx.send_from_address(
                            onward_route.clone(),
                            empty_payload,
                            self.remote_address.clone(),
                        )
                        .await?
                    }
                }

                // We just got correct onward_route, let's send all we had in buffer first
                while let Some(msg) = self.buffer.pop_front() {
                    let msg = TransportMessage::v1(
                        onward_route.clone(),
                        self.remote_address.clone(),
                        msg,
                    );
                    ctx.forward(LocalMessage::new(msg, Vec::new())).await?;
                }
            } else {
                // Send to Tcp stream
                // Create a message buffer with pre-pended length
                let msg = self.prepare_message(msg.payload())?;

                if self.tx.write(&msg).await.is_err() {
                    warn!("Failed to send message to peer {}", self.peer);
                    ctx.stop_worker(ctx.address()).await?;
                }
            }
        }

        Ok(())
    }
}
