use crate::{
    channel::CLUSTER_NAME,
    pipe::{
        PipeBehavior, PipeReceiver, PipeSender, ReceiverConfirm, ReceiverOrdering, SenderConfirm,
    },
    protocols::channel::{ChannelCreationHandshake, ChannelProtocol},
    Context,
};
use ockam_core::compat::collections::VecDeque;
use ockam_core::{
    Address, Any, Decodable, LocalMessage, Result, Route, Routed, TransportMessage, Worker,
};

pub struct ChannelWorker {
    /// Route to the peer channel listener
    listener: Option<Route>,
    /// Internal address used for status messages
    int_addr: Address,
    /// Address of the pipe sender
    tx_addr: Option<Address>,
    /// Sender behaviour in use for this channel
    tx_hooks: PipeBehavior,
    /// A temporary send buffer
    tx_buffer: VecDeque<TransportMessage>,
}

impl ChannelWorker {
    pub async fn initialized(ctx: &Context, addr: Address, tx_addr: Address) -> Result<()> {
        let int_addr = Address::random(0);
        ctx.start_worker(
            vec![addr, int_addr.clone()],
            ChannelWorker {
                listener: None,
                int_addr,
                tx_addr: Some(tx_addr),
                tx_hooks: PipeBehavior::with(SenderConfirm::new()),
                tx_buffer: VecDeque::new(),
            },
        )
        .await
    }

    pub async fn create(ctx: &Context, tx: Address, listener: Route) -> Result<()> {
        let int_addr = Address::random(0);
        ctx.start_worker(
            vec![tx, int_addr.clone()],
            ChannelWorker {
                listener: Some(listener),
                int_addr,
                tx_addr: None,
                tx_hooks: PipeBehavior::with(SenderConfirm::new()),
                tx_buffer: VecDeque::new(),
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

        // If this worker doesn't have a TX address associated yet
        // then we need to start the channel creation handshake
        if self.tx_addr.is_none() && self.listener.is_some() {
            debug!("{}: Initiating channel creation handshake", ctx.address());
            let rx_addr = Address::random(0);
            PipeReceiver::create(
                ctx,
                rx_addr.clone(),
                Address::random(0), // TODO: pass int_addr to handshake handler too
                PipeBehavior::with(ReceiverConfirm).attach(ReceiverOrdering::new()),
            )
            .await?;

            ctx.send(
                self.listener.clone().unwrap(),
                ChannelCreationHandshake(rx_addr),
            )
            .await?;
        }

        // Otherwise we're good to go
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        trace!("Receiving message to address '{}'", msg.msg_addr());
        match msg.msg_addr() {
            addr if addr == self.int_addr => self.handle_internal(ctx, msg).await,
            _ => self.handle_external(ctx, msg).await,
        }
    }
}

impl ChannelWorker {
    /// Handle channel internal messages
    async fn handle_internal(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        trace!("ChannelWorker receiving internal command");
        let mut return_route = msg.return_route();
        let trans = msg.into_transport_message();
        let internal_cmd = ChannelProtocol::decode(&trans.payload)?;

        match internal_cmd {
            // Peer receiver is ready, we can start our sender
            ChannelProtocol::ReceiverReady(rx_addr) => {
                debug!("ChannelWorker handles ReceiverReady message");
                let rx_route = return_route
                    .modify()
                    .pop_back()
                    .append(rx_addr.clone())
                    .into();

                self.tx_addr = Some(Address::random(0));

                PipeSender::create(
                    ctx,
                    rx_route,
                    self.tx_addr.clone().unwrap(),
                    Address::random(0),
                    self.tx_hooks.clone(),
                )
                .await?
            }
        }

        Ok(())
    }

    /// Handle external user messages
    ///
    /// These messages are always `TransportMessage`
    async fn handle_external(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        let mut trans = msg.into_transport_message();
        match self.tx_addr {
            Some(ref tx) => {
                // If we have messages in our output buffer send these first
                for mut msg in core::mem::take(&mut self.tx_buffer) {
                    msg.onward_route.modify().prepend(tx.clone());
                    ctx.forward(LocalMessage::new(msg, vec![])).await?;
                }

                trans.onward_route.modify().prepend(tx.clone());
                ctx.forward(LocalMessage::new(trans, vec![])).await?;
            }
            None => {
                self.tx_buffer.push_back(trans);
            }
        }

        Ok(())
    }
}
