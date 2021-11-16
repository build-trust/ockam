use crate::Context;
use ockam_core::{Address, Any, Result, Routed, Worker};

/// Listen for pipe handshakes and creates PipeReceive workers
pub struct PipeListener;

#[crate::worker]
impl Worker for PipeListener {
    type Context = Context;
    type Message = Any;

    async fn handle_message(&mut self, _: &mut Context, _msg: Routed<Any>) -> Result<()> {
        Ok(())
    }
}

impl PipeListener {
    pub async fn create(ctx: &mut Context, addr: Address) -> Result<()> {
        ctx.start_worker(addr, PipeListener).await?;
        Ok(())
    }
}
