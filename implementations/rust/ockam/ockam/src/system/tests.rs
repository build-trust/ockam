use crate::{Context, SystemHandler, WorkerSystem};
use ockam_core::{Address, Message, Result, Routed, Worker};

#[derive(Default)]
struct TestWorker {
    system: WorkerSystem<Self>,
}

struct MessageHandlerA;

#[ockam_core::async_trait]
impl<C, M> SystemHandler<C, M> for MessageHandlerA
where
    C: Send + 'static,
    M: Message,
{
    async fn initialize(&mut self, _: &mut C) -> Result<Address> {
        Ok("0#my-worker-private-a".into())
    }

    async fn handle_message(&mut self, _: &mut C, _: Routed<M>) -> Result<()> {
        println!("Handling message for address type A");
        Ok(())
    }
}

#[crate::worker]
impl Worker for TestWorker {
    type Context = Context;
    type Message = ();

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.system.attach(ctx, MessageHandlerA).await
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        self.system.handle_message(ctx, msg).await
    }
}

#[crate::test]
async fn send_messages(ctx: &mut Context) -> Result<()> {
    let w = TestWorker::default();

    // Initialise the worker and worker system
    ctx.start_worker("test.worker", w).await?;

    ctx.stop().await
}
