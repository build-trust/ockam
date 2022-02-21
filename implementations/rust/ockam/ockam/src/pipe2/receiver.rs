use crate::{pipe2::PipeSystem, Context, OckamMessage};
use ockam_core::{Address, LocalMessage, Result, Routed, TransportMessage, Worker};

pub struct PipeReceiver {
    system: PipeSystem,
    api_addr: Address,
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
    pub fn new(system: PipeSystem, api_addr: Address) -> Self {
        Self { system, api_addr }
    }
}
