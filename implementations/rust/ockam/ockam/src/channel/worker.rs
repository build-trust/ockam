use crate::{
    channel::CLUSTER_NAME,
    pipe::{HandshakeInit, PipeBehavior, PipeReceiver, PipeSender},
    protocols::{
        channel::ChannelCreationHandshake,
        pipe::internal::{Handshake, InternalCmd},
    },
    Context,
};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::{
    Address, AllowAll, Any, LocalMessage, Mailbox, Mailboxes, Result, Route, Routed, Worker,
};
use ockam_node::WorkerBuilder;

/// Encode the channel creation handshake stage a worker is in
enum WorkerStage {
    Stage1,
    Stage2,
    Finalised,
}

pub struct ChannelWorker {
    stage: WorkerStage,
    /// Route to the peer channel listener
    listener: Option<Route>,
    /// Address of the local pipe sender
    tx_addr: Address,
    /// Address of the local pipe receiver
    rx_addr: Address,
    /// Pair of addresses: one public, one internal
    self_addrs: (Address, Address),
    /// Peer channel worker address
    peer_addr: Address,
    /// Route to the peer's channel worker
    // FIXME please
    peer_routes: Option<((Route, Route), (Route, Route))>,
    /// Sender behaviour in use for this channel
    tx_hooks: PipeBehavior,
    /// Receiver behaviour in use for this channel
    rx_hooks: PipeBehavior,
}

impl ChannelWorker {
    pub async fn stage2(
        ctx: &Context,
        peer_tx_route: (Route, Route),
        peer_rx_route: (Route, Route),
        int_addr: Address,
        peer_addr: Address,
        tx_hooks: PipeBehavior,
        rx_hooks: PipeBehavior,
    ) -> Result<()> {
        let pub_addr = Address::random_local();
        let worker = ChannelWorker {
            stage: WorkerStage::Stage2,
            listener: None,
            tx_addr: Address::random_local(),
            rx_addr: Address::random_local(),
            peer_routes: Some((peer_tx_route, peer_rx_route)),
            self_addrs: (pub_addr.clone(), int_addr.clone()),
            peer_addr,
            tx_hooks,
            rx_hooks,
        };

        // TODO: @ac
        let mailboxes = Mailboxes::new(
            Mailbox::new(int_addr, Arc::new(AllowAll), Arc::new(AllowAll)),
            vec![Mailbox::new(
                pub_addr,
                Arc::new(AllowAll),
                Arc::new(AllowAll),
            )],
        );
        WorkerBuilder::new(worker)
            .with_mailboxes(mailboxes)
            .start(ctx)
            .await?;

        Ok(())
    }

    /// Create a new stage-1 channel worker
    ///
    /// Stage-1 of the handshake consists of creating a PipeReceiver
    /// and initiating the channel creation handshake
    pub async fn stage1(
        ctx: &Context,
        pub_addr: Address,
        listener: Route,
        tx_hooks: PipeBehavior,
        rx_hooks: PipeBehavior,
    ) -> Result<()> {
        let int_addr = Address::random_local();
        let worker = ChannelWorker {
            stage: WorkerStage::Stage1,
            listener: Some(listener),
            peer_routes: None,
            // This is a bit of a hack so that we don't have to
            // create an outgoing message cache in the channel
            // endpoint and can instead use the PipeSender cache
            // mechanism instead
            peer_addr: Address::random_local(),
            rx_addr: Address::random_local(),
            tx_addr: Address::random_local(),
            self_addrs: (pub_addr.clone(), int_addr.clone()),
            tx_hooks,
            rx_hooks,
        };
        // TODO: @ac
        let mailboxes = Mailboxes::new(
            Mailbox::new(int_addr, Arc::new(AllowAll), Arc::new(AllowAll)),
            vec![Mailbox::new(
                pub_addr,
                Arc::new(AllowAll),
                Arc::new(AllowAll),
            )],
        );
        WorkerBuilder::new(worker)
            .with_mailboxes(mailboxes)
            .start(ctx)
            .await?;

        Ok(())
    }
}

#[crate::worker]
impl Worker for ChannelWorker {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(CLUSTER_NAME).await?;

        debug!(
            "Initialise ChannelWorker (pub: {}, int: {})",
            self.self_addrs.0, self.self_addrs.1
        );

        match self.stage {
            WorkerStage::Stage1 => self.init_stage1(ctx).await?,
            WorkerStage::Stage2 => self.init_stage2(ctx).await?,
            WorkerStage::Finalised => unreachable!(),
        }

        // Otherwise we're good to go
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        trace!("Channel receiving message to address '{}'", msg.msg_addr());
        self.handle_external(ctx, msg).await
    }
}

impl ChannelWorker {
    /// Stage 1 init is caused by ChannelBuilder::connect
    ///
    /// 1. Create PipeReceiver
    /// 2. Create PipeSender
    /// 3. Send ChannelCreationhandshake message to listener
    ///
    /// No further initialisation is needed from this worker.  Sender
    /// will be initialised by handshake with peer receiver.  Messages
    /// should be forwarded to the sender, which acts as an output
    /// buffer.
    async fn init_stage1(&mut self, ctx: &mut Context) -> Result<()> {
        debug!(
            "Stage 1 channel init: Sender '{}' and Receiver '{}' pipes",
            self.tx_addr, self.rx_addr
        );

        let rx_int = Address::random_local();
        PipeReceiver::create(
            ctx,
            self.rx_addr.clone(),
            rx_int.clone(),
            self.rx_hooks.clone(),
        )
        .await?;

        let tx_int = Address::random_local();
        PipeSender::uninitialized(
            ctx,
            self.tx_addr.clone(),
            tx_int.clone(),
            None,
            self.tx_hooks.clone(),
        )
        .await?;

        // Send a ChannelCreationHandshake from the internal address,
        // which the peer channel worker will associate with this
        // worker.  That way we can distinguish between messages sent
        // to us by users, and messages sent to us by the PipeReceiver
        debug!("{}: Initiating channel creation handshake", ctx.address());
        ctx.send_from_address(
            self.listener.clone().unwrap(),
            ChannelCreationHandshake {
                channel_addr: self.peer_addr.clone(),
                tx_addr: self.tx_addr.clone(),
                rx_addr: self.rx_addr.clone(),
                tx_int_addr: tx_int,
                rx_int_addr: rx_int,
            },
            self.self_addrs.1.clone(),
        )
        .await?;

        self.stage = WorkerStage::Finalised; // does this matter?
        Ok(())
    }

    /// Stage 2 init is caused by the ChannelListener
    ///
    /// 1. Create PipeReceiver
    /// 2. Create PipeSender and point it at peer-receiver
    /// 3. Send Receiver address to peer-channel
    ///
    /// No further initialisation is needed past this point
    async fn init_stage2(&mut self, ctx: &mut Context) -> Result<()> {
        debug!(
            "Stage 2 channel init: Sender '{}' and Receiver '{}' pipes",
            self.tx_addr, self.rx_addr
        );

        // Get the TX and RX worker routes
        let (route_to_sender, route_to_receiver) = self.peer_routes.clone().unwrap();

        // Create a PipeReceiver
        let rx_int_addr = Address::random_local();
        PipeReceiver::create(
            ctx,
            self.rx_addr.clone(),
            rx_int_addr.clone(),
            self.rx_hooks.clone().attach(HandshakeInit::default()),
        )
        .await?;

        // Create a PipeSender
        PipeSender::create(
            ctx,
            route_to_receiver.0,
            self.tx_addr.clone(),
            Address::random_local(),
            self.tx_hooks.clone(),
        )
        .await?;

        // Send HandshakeInit message to Receiver
        ctx.send(
            rx_int_addr,
            InternalCmd::Handshake(Handshake {
                route_to_sender: route_to_sender.1,
            }),
        )
        .await?;

        self.stage = WorkerStage::Finalised; // does this matter?
        Ok(())
    }

    /// Handle external user messages
    ///
    /// These messages are always `TransportMessage` and should be
    /// forwarded to the PipeSender.  If the sender isn't fully
    /// initialised yet it can buffer messages for us.
    async fn handle_external(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        let msg_addr = msg.msg_addr();
        let mut trans = msg.into_transport_message();
        match msg_addr {
            // Message received to public address -- forward to PipeSender
            addr if addr == self.self_addrs.0 => {
                trans
                    .onward_route
                    .modify()
                    .pop_front()
                    .prepend(self.peer_addr.clone())
                    .prepend(self.tx_addr.clone());
            }
            // Message received to internal address -- forward to user
            addr if addr == self.self_addrs.1 => {
                trans.onward_route.modify().pop_front();
                trans
                    .return_route
                    .modify()
                    .prepend(self.self_addrs.0.clone());
            }
            addr => warn!("Received invalid message to address {}", addr),
        }

        // Forward message
        ctx.forward(LocalMessage::new(trans, vec![])).await.unwrap();
        Ok(())
    }
}
