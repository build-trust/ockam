use crate::{
    pipe::{BehaviorHook, PipeModifier},
    protocols::pipe::{
        internal::{Handshake, InternalCmd},
        PipeMessage,
    },
    Context,
};
use ockam_core::compat::boxed::Box;
use ockam_core::{Address, Result, Route};

#[derive(Clone, Default)]
pub struct HandshakeInit(bool);

#[ockam_core::async_trait]
impl BehaviorHook for HandshakeInit {
    async fn on_internal(
        &mut self,
        _: Address,
        _: Route,
        ctx: &mut Context,
        msg: &InternalCmd,
    ) -> Result<()> {
        if let (InternalCmd::Handshake(Handshake { route_to_sender }), false) = (msg, self.0) {
            debug!("Sending InitSender request to {:?}", route_to_sender);
            ctx.send(route_to_sender.clone(), InternalCmd::InitSender)
                .await?;
            self.0 = true;
        }

        Ok(())
    }

    async fn on_external(
        &mut self,
        _: Address,
        _: Route,
        _: &mut Context,
        _: &PipeMessage,
    ) -> Result<PipeModifier> {
        Ok(PipeModifier::None)
    }
}
