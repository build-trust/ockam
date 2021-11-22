use crate::{
    pipe::{BehaviorHook, PipeBehavior, PipeModifier, PipeReceiver},
    protocols::pipe::{
        internal::{HandShake, InternalCmd},
        PipeMessage,
    },
    Context,
};
use ockam_core::compat::boxed::Box;
use ockam_core::{Address, Result, Route, Routed, Worker};

/// Listen for pipe handshakes and creates PipeReceive workers
pub struct PipeListener {
    hooks: PipeBehavior,
}

#[crate::worker]
impl Worker for PipeListener {
    type Context = Context;
    type Message = InternalCmd;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::pipe::CLUSTER_NAME).await
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<InternalCmd>) -> Result<()> {
        let route_to_sender = msg.return_route();
        match msg.body() {
            InternalCmd::InitHandshake => {
                info!("Creating new PipeReceiver for incoming handshake");

                // Create a new pipe receiver with a modified behavioral stack
                let recv_addr = Address::random(0);
                let int_addr = Address::random(0);
                let hooks = self.hooks.clone().attach(HandshakeInit(false));
                PipeReceiver::create(ctx, recv_addr.clone(), int_addr.clone(), hooks).await?;

                // Then send it the handshake message
                ctx.send(
                    vec![int_addr],
                    InternalCmd::Handshake(HandShake { route_to_sender }),
                )
                .await?;
            }
            cmd => debug!("Ignoring invalid cmd: {:?}", cmd),
        }

        Ok(())
    }
}

impl PipeListener {
    /// Create pipe creation listener with explicit behavior hooks
    pub async fn create_with_behavior(
        ctx: &mut Context,
        addr: Address,
        hooks: PipeBehavior,
    ) -> Result<()> {
        ctx.start_worker(addr, PipeListener { hooks }).await?;
        Ok(())
    }

    /// Create pipe creation listener with empty behavior hooks
    pub async fn create(ctx: &mut Context, addr: Address) -> Result<()> {
        Self::create_with_behavior(ctx, addr, PipeBehavior::empty()).await
    }
}

#[derive(Clone)]
struct HandshakeInit(bool);

#[ockam_core::async_trait]
impl BehaviorHook for HandshakeInit {
    async fn on_internal(
        &mut self,
        _: Address,
        _: Route,
        ctx: &mut Context,
        msg: &InternalCmd,
    ) -> Result<()> {
        if let (InternalCmd::Handshake(HandShake { route_to_sender }), false) = (msg, self.0) {
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
