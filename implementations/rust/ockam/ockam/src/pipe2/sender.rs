//! Pipe2 Send worker

use crate::{pipe2::PipeSystem, Context, OckamMessage};
use ockam_core::{
    compat::{boxed::Box, collections::VecDeque},
    Address, Any, Result, Route, Routed, Worker,
};

pub enum PeerRoute {
    Peer(Route),
    Listener(Route, Address),
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

        // If the worker was initialised with a "Listener" peer we
        // subsequently start the handshake to create a pipe receiver
        if let Some(PeerRoute::Listener(ref route, ref addr)) = self.peer {
            debug!("Sending pipe2 handshake request to listener: {}", route);
            ctx.send_from_address(route.clone(), OckamMessage::new(Any)?, addr.clone())
                .await?;
        }

        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        debug!(
            "PipeSender '{}': handling incoming message to {}",
            ctx.address(),
            msg.onward_route()
        );

        match (msg.msg_addr(), self.peer.as_ref()) {
            (ref addr, Some(&PeerRoute::Listener(_, ref _self))) if addr == _self => {
                let return_route = msg.return_route();
                self.peer = Some(PeerRoute::Peer(return_route.clone()));

                for msg in core::mem::replace(&mut self.out_buf, vec![].into()) {
                    ctx.send(return_route.clone(), msg).await?;
                }

                Ok(())
            }

            // Messages sent by users
            (addr, _) if addr == self.api_addr => self.handle_api_msg(ctx, msg).await,

            // The end point of the worker system routes
            (addr, _) if addr == self.fin_addr => {
                self.handle_fin_msg(ctx, OckamMessage::from_any(msg)?).await
            }

            // These messages are most likely intra-system
            _ => self.system.handle_message(ctx, msg.cast()?).await,
        }
    }
}

impl PipeSender {
    pub fn new(system: PipeSystem, peer: PeerRoute, api_addr: Address, fin_addr: Address) -> Self {
        Self {
            out_buf: VecDeque::default(),
            peer: Some(peer),
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

        // Either dispatch a message into the worker system or to the "fin" address
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
            Some(PeerRoute::Peer(ref peer)) => ctx.send(peer.clone(), msg).await?,

            // If field is None or PeerRoute::Listener we are not yet
            // ready to send messages and store them for later
            _ => self.out_buf.push_back(msg),
        }

        Ok(())
    }
}
