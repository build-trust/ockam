use crate::{Context, OckamMessage, WorkerSystem};
use ockam_core::{Address, Any, LocalMessage, Result, Routed, TransportMessage, Worker};

pub struct PipeReceiver {
    system: WorkerSystem<Context, OckamMessage>,
    int_addr: Address,
}

#[crate::worker]
impl Worker for PipeReceiver {
    type Context = Context;
    type Message = OckamMessage;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::pipe2::CLUSTER_NAME).await?;
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<OckamMessage>) -> Result<()> {
        let inner: TransportMessage = msg.body().data()?;
        ctx.forward(LocalMessage::new(inner, vec![])).await
    }
}

impl PipeReceiver {
    pub fn new(int_addr: Address) -> Self {
        Self {
            system: WorkerSystem::default(),
            int_addr,
        }
    }
}
