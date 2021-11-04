use crate::{Address, LocalMessage, Message, NodeContext, Result, Routed};

/// A message wraper type that allows users to cancel message receival
///
/// A worker can block in place to wait for a message.  If the next
/// message is not the desired type, it can be cancelled which
/// re-queues it into the mailbox.
pub struct Cancel<'ctx, C, M> {
    inner: M,
    local_msg: LocalMessage,
    addr: Address,
    ctx: &'ctx C,
}

impl<'ctx, C: NodeContext, M: Message> Cancel<'ctx, C, M> {
    /// Create a new `Cancel`. This should only be used inside node
    /// implementations.
    #[doc(hidden)]
    pub fn new(inner: M, local_msg: LocalMessage, addr: Address, ctx: &'ctx C) -> Self {
        Self {
            inner,
            local_msg,
            addr,
            ctx,
        }
    }

    /// Cancel this message
    pub async fn cancel(self) -> Result<()> {
        self.ctx.forward(self.local_msg).await
    }

    /// Consume the Cancel wrapper to take the underlying message
    ///
    /// After calling this function it is no longer possible to
    /// re-queue the message into the worker mailbox.
    pub fn take(self) -> Routed<M> {
        Routed::new(self.inner, self.addr, self.local_msg)
    }
}

impl<'ctx, C: NodeContext, M: Message> core::ops::Deref for Cancel<'ctx, C, M> {
    type Target = M;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'ctx, C: NodeContext, M: Message + PartialEq> PartialEq<M> for Cancel<'ctx, C, M> {
    fn eq(&self, o: &M) -> bool {
        &self.inner == o
    }
}

impl<'ctx, C: NodeContext, M: Message + core::fmt::Debug> core::fmt::Debug for Cancel<'ctx, C, M> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<'ctx, C: NodeContext, M: Message + core::fmt::Display> core::fmt::Display
    for Cancel<'ctx, C, M>
{
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.inner.fmt(f)
    }
}
