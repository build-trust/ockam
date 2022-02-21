//! Pipe2 Send worker

use crate::{pipe2::PipeSystem, Context, OckamMessage};
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
    system: PipeSystem,
    out_buf: VecDeque<OckamMessage>,
    peer: Option<PeerRoute>,
    api_addr: Address,
    fin_addr: Address,
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
        match msg.msg_addr() {
            // Messages sent by users
            addr if addr == self.api_addr => self.handle_api_msg(ctx, msg).await,
            // The end point of the worker system routes
            addr if addr == self.fin_addr => {
                self.handle_fin_msg(ctx, OckamMessage::from_any(msg)?).await
            }
            // These messages are most likely intra-system
            _ => self.system.handle_message(ctx, msg.cast()?).await,
        }
    }
}

impl PipeSender {
    pub fn new(system: PipeSystem, peer: Route, api_addr: Address, fin_addr: Address) -> Self {
        Self {
            out_buf: VecDeque::default(),
            peer: Some(PeerRoute::Peer(peer)),
            system,
            api_addr,
            fin_addr,
        }
    }

    /// An API message is a user-payload that was sent to this sender.
    /// As the first step we wrap the user message in an OckamMessage
    /// type and then dispatch it into the worker system.
    async fn handle_api_msg(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        trace!(
            "PipeSender '{}' handling initial user message stage...",
            ctx.address()
        );

        // Grab data from the Routed wrapper
        let msg_addr = msg.msg_addr().clone();
        let onward_route = msg.onward_route();
        let return_route = msg.return_route();

        // Grab the internal message and edit its route info
        let mut inner = msg.into_transport_message();
        inner.onward_route.modify().pop_front();

        // Then wrap the message in an OckamMessage and dispatch
        let ockam_msg = OckamMessage::new(inner)?;

        if self.system.is_empty() {
            self.handle_fin_msg(ctx, ockam_msg).await?;
        } else {
            let routed = ockam_msg.into_routed(msg_addr, onward_route, return_route)?;
            self.system.dispatch_entry(ctx, routed).await?;
        }

        Ok(())
    }

    /// A "fin" message just came out of the worker system and can
    /// simply be sent to our remote peer.  Any additional behaviour
    /// and encodings has now been set-up.
    async fn handle_fin_msg(&mut self, ctx: &mut Context, msg: OckamMessage) -> Result<()> {
        trace!(
            "PipeSender '{}' handling final user message stage...",
            ctx.address()
        );

        // TODO: get the address we're supposed to use from the
        // OckamMessage global metadata section and remove it
        match self.peer {
            Some(PeerRoute::Peer(ref peer)) => {
                // If messages are in out_buf then we send those first
                // TODO: maybe move this to the handshake logic?
                for msg in core::mem::replace(&mut self.out_buf, vec![].into()) {
                    ctx.send(peer.clone(), msg).await?;
                }

                // Then we send the actual message we handled
                ctx.send(peer.clone(), msg).await?;
            }
            // If field is None or PeerRoute::Listener we are not yet
            // ready to send messages and store them for later
            _ => {
                self.out_buf.push_back(msg);
            }
        }

        Ok(())
    }
}
