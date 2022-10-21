use crate::{
    delay::DelayedEvent,
    pipe::behavior::{BehaviorHook, PipeModifier},
    protocols::pipe::{
        internal::{Ack, InternalCmd, Resend},
        PipeMessage,
    },
    Context,
};
use ockam_core::compat::boxed::Box;
use ockam_core::{async_trait, compat::collections::BTreeMap, Address, Result, Route};

#[derive(Default, Clone)]
pub struct SenderConfirm {
    /// A set of message indices not confirmed yet
    on_route: BTreeMap<u64, PipeMessage>,
}

impl SenderConfirm {
    pub fn new() -> Self {
        Self {
            on_route: BTreeMap::new(),
        }
    }
}

#[async_trait]
impl BehaviorHook for SenderConfirm {
    async fn on_external(
        &mut self,
        this: Address,
        _: Route,
        ctx: &mut Context,
        msg: &PipeMessage,
    ) -> Result<PipeModifier> {
        self.on_route.insert(msg.index.u64(), msg.clone());

        DelayedEvent::new(
            ctx,
            this.into(),
            InternalCmd::Resend(Resend {
                idx: msg.index.u64(),
            }),
        )
        .await?
        .with_seconds(5)
        .spawn();

        Ok(PipeModifier::None)
    }

    async fn on_internal(
        &mut self,
        this: Address,
        peer: Route,
        ctx: &mut Context,
        msg: &InternalCmd,
    ) -> Result<()> {
        match msg {
            InternalCmd::Resend(Resend { idx }) => match self.on_route.remove(idx) {
                Some(msg) => {
                    debug!(
                        "Received message index '{}' timeout: resending to peer {}",
                        idx, peer
                    );

                    // First re-queue another timeout event
                    self.on_external(this.clone(), peer.clone(), ctx, &msg)
                        .await?;

                    // Then actually re-send the message
                    ctx.send(peer, msg).await?;
                }
                None => trace!("Received timeout for message, but message was acknowledged"),
            },
            InternalCmd::Ack(Ack { idx }) => {
                debug!("Received pipe delivery ACK for index {}", idx);
                self.on_route.remove(idx);
            }
            cmd => trace!("SenderResend behavior ignoring {:?}", cmd),
        }

        Ok(())
    }
}

///
#[derive(Clone)]
pub struct ReceiverConfirm;

#[async_trait]
impl BehaviorHook for ReceiverConfirm {
    async fn on_external(
        &mut self,
        _: Address,
        sender: Route,
        ctx: &mut Context,
        msg: &PipeMessage,
    ) -> Result<PipeModifier> {
        debug!(
            "Sending delivery ACK for message index '{}'",
            msg.index.u64()
        );
        ctx.send(
            sender,
            InternalCmd::Ack(Ack {
                idx: msg.index.u64(),
            }),
        )
        .await
        .map(|_| PipeModifier::None)
    }

    async fn on_internal(
        &mut self,
        _: Address,
        _: Route,
        _: &mut Context,
        _: &InternalCmd,
    ) -> Result<()> {
        Ok(())
    }
}
