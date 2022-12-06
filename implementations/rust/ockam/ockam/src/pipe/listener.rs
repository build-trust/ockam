use crate::{
    pipe::{HandshakeInit, PipeBehavior, PipeReceiver},
    protocols::pipe::internal::{Handshake, InternalCmd},
    Context,
};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::{Address, AllowAll, Result, Routed, Worker};

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
                let recv_addr = Address::random_local();
                let int_addr = Address::random_local();
                let hooks = self.hooks.clone().attach(HandshakeInit::default());
                PipeReceiver::create(ctx, recv_addr.clone(), int_addr.clone(), hooks).await?;

                // Then send it the handshake message
                ctx.send(
                    int_addr,
                    InternalCmd::Handshake(Handshake { route_to_sender }),
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
        ctx.start_worker(
            addr,
            PipeListener { hooks },
            Arc::new(AllowAll), // FIXME: @ac
            Arc::new(AllowAll), // FIXME: @ac
        )
        .await?;
        Ok(())
    }

    /// Create pipe creation listener with empty behavior hooks
    pub async fn create(ctx: &mut Context, addr: Address) -> Result<()> {
        Self::create_with_behavior(ctx, addr, PipeBehavior::empty()).await
    }
}
