use crate::{
    monotonic::Monotonic,
    pipe::PipeBehavior,
    protocols::pipe::{internal::InternalCmd, PipeMessage},
    Context,
};
use ockam_core::compat::{boxed::Box, collections::VecDeque};
use ockam_core::{Address, Any, Mailbox, Mailboxes, Result, Route, Routed, Worker};
use ockam_node::WorkerBuilder;

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
    index: Monotonic,
    out_buf: VecDeque<PipeMessage>,
    peer: Option<PeerRoute>,
    int_addr: Address,
    hooks: PipeBehavior,
}

#[ockam_core::worker]
impl Worker for PipeSender {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::pipe::CLUSTER_NAME).await?;
        if let Some(PeerRoute::Listener(ref route)) = self.peer {
            ctx.send_from_address(
                route.clone(),
                InternalCmd::InitHandshake,
                self.int_addr.clone(),
            )
            .await?
        }

        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        match msg.msg_addr() {
            addr if addr == self.int_addr => self.handle_internal(ctx, msg).await?,
            _ => self.handle_external(ctx, msg).await?,
        };

        Ok(())
    }
}

impl PipeSender {
    /// Create a PipeSender with a public and "internal" address
    pub async fn create(
        ctx: &mut Context,
        peer: Route,
        addr: Address,
        int_addr: Address,
        hooks: PipeBehavior,
    ) -> Result<()> {
        let worker = Self {
            // Ordered pipes expect a 1-indexed message
            index: Monotonic::from(1),
            out_buf: VecDeque::new(),
            peer: Some(PeerRoute::Peer(peer)),
            int_addr: int_addr.clone(),
            hooks,
        };

        // TODO: @ac
        let mailboxes = Mailboxes::new(Mailbox::deny_all(addr), vec![Mailbox::deny_all(int_addr)]);
        WorkerBuilder::with_mailboxes(mailboxes, worker)
            .start(ctx)
            .await?;

        Ok(())
    }

    /// Create a sender without a peer route
    ///
    /// To fully be able to use this pipe sender it needs to receive
    /// an initialisation message from a pipe receiver.
    pub async fn uninitialized(
        ctx: &mut Context,
        addr: Address,
        int_addr: Address,
        listener: Option<Route>,
        hooks: PipeBehavior,
    ) -> Result<()> {
        let worker = Self {
            index: Monotonic::from(1),
            out_buf: VecDeque::new(),
            peer: listener.map(PeerRoute::Listener),
            int_addr: int_addr.clone(),
            hooks,
        };
        // TODO: @ac
        let mailboxes = Mailboxes::new(Mailbox::deny_all(addr), vec![Mailbox::deny_all(int_addr)]);
        WorkerBuilder::with_mailboxes(mailboxes, worker)
            .start(ctx)
            .await?;

        Ok(())
    }

    /// Handle internal command payloads
    async fn handle_internal(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        trace!("PipeSender receiving internal command");
        let return_route = msg.return_route();
        let trans = msg.into_transport_message();
        let internal_cmd = InternalCmd::from_transport(&trans)?;

        // Either run all internal message hooks OR fully set-up this sender
        match self.peer {
            Some(PeerRoute::Peer(ref peer)) => {
                self.hooks
                    .internal_all(self.int_addr.clone(), peer.clone(), ctx, &internal_cmd)
                    .await?;
            }
            _ => match internal_cmd {
                InternalCmd::InitSender => {
                    debug!("Initialise pipe sender for route {:?}", return_route);
                    self.peer = Some(PeerRoute::Peer(return_route.clone()));

                    // Send out the out_buffer
                    for msg in core::mem::take(&mut self.out_buf) {
                        send_pipe_msg(
                            &mut self.hooks,
                            ctx,
                            self.int_addr.clone(),
                            return_route.clone(),
                            msg,
                        )
                        .await?;
                    }
                }
                cmd => warn!(
                    "Received internal command '{:?}' for invalid state sender",
                    cmd
                ),
            },
        }

        Ok(())
    }

    /// Handle external user messages
    async fn handle_external(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        // First manipulate the onward_route state
        let mut msg = msg.into_transport_message();
        msg.onward_route.modify().pop_front();

        debug!(
            "Pipe sender '{:?}' dispatch {:?} -> {:?}",
            ctx.address(),
            self.peer.as_ref().map(|p| p.peer()),
            msg.onward_route
        );

        // Then pack TransportMessage into PipeMessage
        let index = self.index.next() as u64;
        let pipe_msg = PipeMessage::from_transport(index, msg)?;

        // Check if this sender has been fully initialised already
        match self.peer {
            Some(PeerRoute::Peer(ref peer)) => {
                send_pipe_msg(
                    &mut self.hooks,
                    ctx,
                    self.int_addr.clone(),
                    peer.clone(),
                    pipe_msg,
                )
                .await
            }
            _ => {
                debug!("Queue message into output buffer...");
                self.out_buf.push_back(pipe_msg);
                Ok(())
            }
        }
    }
}

async fn send_pipe_msg(
    hooks: &mut PipeBehavior,
    ctx: &mut Context,
    int_addr: Address,
    peer: Route,
    msg: PipeMessage,
) -> Result<()> {
    // Before we send we give all hooks a chance to run
    if let crate::pipe::PipeModifier::Drop = hooks
        .external_all(int_addr.clone(), peer.clone(), ctx, &msg)
        .await?
    {
        // Return early to prevent message sending if the
        // behaviour stack has determined to drop the message.
        return Ok(());
    }

    // Then send the message from our internal address so the
    // receiver can send any important messages there
    ctx.send_from_address(peer, msg, int_addr).await
}
