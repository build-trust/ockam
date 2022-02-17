//! Pipe2 Send worker

use crate::{Context, OckamMessage, SystemHandler, WorkerSystem};
use ockam_core::{compat::collections::VecDeque, Address, Any, Result, Route, Routed, Worker};

enum PeerRoute {
    Peer(Route),
    Listener(Route),
}

impl PeerRoute {
    fn peer(&self) -> &Route {
        match self {
            Self::Peer(ref p) => p,
            Self::Listener(ref l) => l,
        }
    }
}

pub struct PipeSender {
    system: WorkerSystem<Context, OckamMessage>,
    out_buf: VecDeque<OckamMessage>,
    peer: Option<PeerRoute>,
    api_addr: Address,
    int_addr: Address,
}

#[crate::worker]
impl Worker for PipeSender {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::pipe2::CLUSTER_NAME).await?;
        // TODO: start handshake here
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        // For now we only send messages onward if they were sent to the public address
        if msg.msg_addr() == self.api_addr {
            debug!("Receiving message for pipe sender API address...");

            // Turn the user message into an OckamMessage
            let mut inner = msg.into_transport_message();
            inner.onward_route.modify().pop_front();
            let ockam_msg = OckamMessage::new(inner)?;

            // TODO: check worker system here

            match self.peer {
                Some(PeerRoute::Peer(ref peer)) => {
                    ctx.send_from_address(peer.clone(), ockam_msg, self.int_addr.clone())
                        .await?;
                }
                _ => self.out_buf.push_back(ockam_msg),
            }
        }

        Ok(())
    }
}

impl PipeSender {
    pub fn new(peer: Route, api_addr: Address, int_addr: Address) -> Self {
        Self {
            system: WorkerSystem::default(),
            out_buf: VecDeque::default(),
            peer: Some(PeerRoute::Peer(peer)),
            api_addr,
            int_addr,
        }
    }

    async fn send(&self, peer: &Route, ctx: &mut Context, msg: OckamMessage) -> Result<()> {
        ctx.send_from_address(peer.clone(), msg, self.int_addr.clone())
            .await
    }
}
