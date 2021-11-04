use crate::{async_trait, compat::boxed::Box, NodeContext};
use crate::{Address, AsyncTryClone, Message, Result};

/// Wrapper for some `NodeContext` and `Address`
pub struct Handle<C> {
    ctx: C,
    address: Address,
}

#[async_trait]
impl<C: NodeContext> AsyncTryClone for Handle<C> {
    async fn async_try_clone(&self) -> Result<Self> {
        Ok(Handle {
            ctx: self.ctx.new_context(Address::random(0)).await?,
            address: self.address.clone(),
        })
    }
}

impl<C: NodeContext> Handle<C> {
    /// Create a new `Handle` from a `Context` and `Address`
    pub fn new(ctx: C, address: Address) -> Self {
        Handle { ctx, address }
    }

    /// Asynchronously sends a message
    pub async fn cast<M: Message + Send + 'static>(&self, msg: M) -> Result<()> {
        self.ctx.send(self.address.clone(), msg).await
    }

    /// Asynchronously sends and receiving a message using a new `Context`
    pub async fn call<I: Message + Send + 'static, O: Message + Send + 'static>(
        &self,
        msg: I,
    ) -> Result<O> {
        let mut ctx = self.ctx.new_context(Address::random(0)).await?;
        ctx.send(self.address.clone(), msg).await?;
        let msg = ctx.receive::<O>().await?;
        Ok(msg.take().body())
    }
}

impl<C: NodeContext> Handle<C> {
    /// Gets inner `Context` as reference
    pub fn ctx(&self) -> &C {
        &self.ctx
    }

    /// Gets inner `Context` as mutable reference
    pub fn ctx_mut(&mut self) -> &mut C {
        &mut self.ctx
    }

    /// Gets inner `Address` as reference
    pub fn address(&self) -> &Address {
        &self.address
    }

    /// Gets inner `Address` as mutable reference
    pub fn address_mut(&mut self) -> &mut Address {
        &mut self.address
    }
}
