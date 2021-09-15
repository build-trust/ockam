use ockam_core::{Address, Message, Result};

use crate::{block_future, Context};

/// Wrapper for `Context` and `Address`
pub struct Handle {
    ctx: Context,
    address: Address,
}

impl Clone for Handle {
    fn clone(&self) -> Self {
        block_future(&self.ctx.runtime(), async move {
            Handle {
                ctx: self
                    .ctx
                    .new_context(Address::random(0))
                    .await
                    .expect("new_context failed"),
                address: self.address.clone(),
            }
        })
    }
}

impl Handle {
    /// Create a new `Handle` from a `Context` and `Address`
    pub fn new(ctx: Context, address: Address) -> Self {
        Handle { ctx, address }
    }

    /// Asynchronously sends a message
    pub async fn async_cast<M: Message + Send + 'static>(&self, msg: M) -> Result<()> {
        self.ctx.send(self.address.clone(), msg).await
    }

    /// Sends a message that blocks current `Worker` without blocking the executor.
    pub fn cast<M: Message + Send + 'static>(&self, msg: M) -> Result<()> {
        block_future(
            &self.ctx.runtime(),
            async move { self.async_cast(msg).await },
        )
    }

    /// Asynchronously sends and receiving a message using a new `Context`
    pub async fn async_call<I: Message + Send + 'static, O: Message + Send + 'static>(
        &self,
        msg: I,
    ) -> Result<O> {
        let mut ctx = self
            .ctx
            .new_context(Address::random(0))
            .await
            .expect("new_context failed");
        ctx.send(self.address.clone(), msg).await?;
        let msg = ctx.receive::<O>().await?;
        Ok(msg.take().body())
    }

    /// Send and receiving a message that blocks current `Worker` without blocking the executor.
    pub fn call<I: Message + Send + 'static, O: Message + Send + 'static>(
        &self,
        msg: I,
    ) -> Result<O> {
        block_future(
            &self.ctx.runtime(),
            async move { self.async_call(msg).await },
        )
    }

}