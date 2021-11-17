use crate::{
    delay::DelayedEvent,
    pipe::behavior::BehaviorHook,
    protocols::pipe::{
        internal::{Ack, InternalCmd, Resend},
        PipeMessage,
    },
};
use ockam_core::{async_trait, compat::collections::BTreeMap, Address, Result, Route};
use ockam_node::Context;

pub struct SenderConfirm {
    /// A set of message indices not confirmed yet
    on_route: BTreeMap<u64, PipeMessage>,
}

#[async_trait]
impl BehaviorHook for SenderConfirm {
    async fn on_external(
        &mut self,
        this: Address,
        _: Route,
        ctx: &mut Context,
        msg: &PipeMessage,
    ) -> Result<()> {
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

        Ok(())
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
                        "Received message timeout: resending payload to peer {}",
                        peer
                    );

                    // First re-queue another timeout event
                    self.on_external(this.clone(), peer.clone(), ctx, &msg)
                        .await?;

                    // Then actually re-send the message
                    ctx.send(peer, msg).await?;
                }
                None => trace!("Received timeout for message, but message was acknowleged"),
            },
            InternalCmd::Ack(Ack { idx }) => {
                debug!("Received pipe delivery ACK");
                self.on_route.remove(idx);
            }
            _ => todo!(),
        }

        Ok(())
    }
}
