use crate::Context;
use core::fmt::{self, Debug, Display, Formatter};
use ockam_core::{Address, LocalMessage, Message, Routed};

/// A message wraper type that allows users to cancel message receival
///
/// A worker can block in place to wait for a message.  If the next
/// message is not the desired type, it can be cancelled which
/// re-queues it into the mailbox.
pub struct Cancel<'ctx, M: Message> {
    inner: M,
    local_msg: LocalMessage,
    addr: Address,
    ctx: &'ctx Context,
}

impl<'ctx, M: Message> Cancel<'ctx, M> {
    pub(crate) fn new(
        inner: M,
        local_msg: LocalMessage,
        addr: Address,
        ctx: &'ctx Context,
    ) -> Self {
        Self {
            inner,
            local_msg,
            addr,
            ctx,
        }
    }

    /// Cancel this message
    pub async fn cancel(self) -> ockam_core::Result<()> {
        self.ctx.forward(self.local_msg).await?;

        Ok(())
    }

    /// Consume the Cancel wrapper to take the underlying message
    ///
    /// After calling this function it is no longer possible to
    /// re-queue the message into the worker mailbox.
    pub fn take(self) -> Routed<M> {
        Routed::new(self.inner, self.addr, self.local_msg)
    }
}

impl<'ctx, M: Message> core::ops::Deref for Cancel<'ctx, M> {
    type Target = M;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'ctx, M: Message + PartialEq> PartialEq<M> for Cancel<'ctx, M> {
    fn eq(&self, o: &M) -> bool {
        &self.inner == o
    }
}

impl<'ctx, M: Message + Debug> Debug for Cancel<'ctx, M> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<'ctx, M: Message + Display> Display for Cancel<'ctx, M> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}
