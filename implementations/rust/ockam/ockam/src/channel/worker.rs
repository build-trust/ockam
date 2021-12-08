use crate::{
    channel::CLUSTER_NAME,
    pipe::{HandshakeInit, PipeBehavior, PipeReceiver, PipeSender},
    protocols::{
        channel::ChannelCreationHandshake,
        pipe::internal::{Handshake, InternalCmd},
    },
    Context,
};
use ockam_core::{Address, Any, LocalMessage, Result, Route, Routed, Worker};

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
    /// Route to the peer's channel worker
    peer_routes: Option<(Route, Route)>,
    /// Sender behaviour in use for this channel
    tx_hooks: PipeBehavior,
    /// Receiver behaviour in use for this channel
    rx_hooks: PipeBehavior,
}

impl ChannelWorker {
    pub async fn stage2(
        ctx: &Context,
        peer_tx_route: Route,
        peer_rx_route: Route,
        tx_hooks: PipeBehavior,
        rx_hooks: PipeBehavior,
    ) -> Result<()> {
        ctx.start_worker(
            Address::random(0),
            ChannelWorker {
                stage: WorkerStage::Stage2,
                listener: None,
                tx_addr: Address::random(0),
                rx_addr: Address::random(0),
                peer_routes: Some((peer_tx_route, peer_rx_route)),
                tx_hooks,
                rx_hooks,
            },
        )
        .await
    }

    /// Create a new stage-1 channel worker
    ///
    /// Stage-1 of the handshake consists of creating a PipeReceiver
    /// and initiating the channel creation handshake
    pub async fn stage1(
        ctx: &Context,
        addr: Address,
        listener: Route,
        tx_hooks: PipeBehavior,
        rx_hooks: PipeBehavior,
    ) -> Result<()> {
        ctx.start_worker(
            addr,
            ChannelWorker {
                stage: WorkerStage::Stage1,
                listener: Some(listener),
                peer_routes: None,
                rx_addr: Address::random(0),
                tx_addr: Address::random(0),
                tx_hooks,
                rx_hooks,
            },
        )
        .await
    }
}

#[crate::worker]
impl Worker for ChannelWorker {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(CLUSTER_NAME).await?;

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
        debug!("{}: Initiating channel creation handshake", ctx.address());
        PipeReceiver::create(
            ctx,
            self.rx_addr.clone(),
            Address::random(0),    // TODO: pass int_addr to handshake handler too
            self.rx_hooks.clone(), // PipeBehavior::with(ReceiverConfirm).attach(ReceiverOrdering::new()),
        )
        .await?;

        PipeSender::uninitialized(
            ctx,
            self.tx_addr.clone(),
            Address::random(0),
            Route::new().into(),
            self.tx_hooks.clone(),
        )
        .await?;

        ctx.send(
            self.listener.clone().unwrap(),
            ChannelCreationHandshake(self.rx_addr.clone(), self.tx_addr.clone()),
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
        // Get the TX and RX worker routes
        let (route_to_sender, route_to_receiver) = self.peer_routes.clone().unwrap();

        // Create a PipeReceiver
        PipeReceiver::create(
            ctx,
            self.rx_addr.clone(),
            Address::random(0),
            self.rx_hooks.clone().attach(HandshakeInit::default()),
            // PipeBehavior::with(ReceiverConfirm)
            //     .attach(ReceiverOrdering::new())
            //     .attach(HandshakeInit::default()),
        )
        .await?;

        // Create a PipeSender
        PipeSender::create(
            ctx,
            route_to_receiver,
            self.tx_addr.clone(),
            Address::random(0),
            self.tx_hooks.clone(),
        )
        .await?;

        // Send HandshakeInit message to Receiver
        ctx.send(
            self.rx_addr.clone(),
            InternalCmd::Handshake(Handshake { route_to_sender }),
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
        let mut trans = msg.into_transport_message();
        trans.onward_route.modify().prepend(self.tx_addr.clone());
        ctx.forward(LocalMessage::new(trans, vec![])).await
    }
}
