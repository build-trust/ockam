use crate::{relay::RelayMessage, Context};
use ockam_core::{Address, Message, Routed, TransportMessage};
use std::fmt::{self, Debug, Display, Formatter};
use tokio::sync::mpsc::{Receiver, Sender};

/// A mailbox for encoded messages
///
/// Message type information can't be exposed at this stage because
/// they need to either be typed in the `Relay` or in the worker's
/// [`Context`](crate::Context).
#[derive(Debug)]
pub struct Mailbox {
    rx: Receiver<RelayMessage>,
    tx: Sender<RelayMessage>,
}

impl Mailbox {
    pub fn new(rx: Receiver<RelayMessage>, tx: Sender<RelayMessage>) -> Self {
        Self { rx, tx }
    }

    pub fn sender(&self) -> Sender<RelayMessage> {
        self.tx.clone()
    }

    /// Get the next message from the mailbox
    pub async fn next(&mut self) -> Option<RelayMessage> {
        self.rx.recv().await
    }

    /// If a message wasn't expected, requeue it
    pub async fn requeue(&self, msg: RelayMessage) {
        self.tx.send(msg).await.unwrap();
    }
}

/// A message wraper type that allows users to cancel message receival
///
/// A worker can block in place to wait for a message.  If the next
/// message is not the desired type, it can be cancelled which
/// re-queues it into the mailbox.
pub struct Cancel<'ctx, M: Message> {
    inner: M,
    trans: TransportMessage,
    addr: Address,
    ctx: &'ctx Context,
}

impl<'ctx, M: Message> Cancel<'ctx, M> {
    pub(crate) fn new(
        inner: M,
        trans: TransportMessage,
        addr: Address,
        ctx: &'ctx Context,
    ) -> Self {
        Self {
            inner,
            trans,
            addr,
            ctx,
        }
    }

    /// Cancel this message
    pub async fn cancel(self) {
        let ctx = self.ctx;
        let onward = self.trans.onward.clone();
        ctx.mailbox
            .requeue(RelayMessage::direct(self.addr, self.trans, onward))
            .await;
    }

    /// Consume the Cancel wrapper to take the underlying message
    ///
    /// After calling this function it is no longer possible to
    /// re-queue the message into the worker mailbox.
    pub fn take(self) -> Routed<M> {
        Routed::new(self.inner, self.trans.return_, self.trans.onward)
    }
}

impl<'ctx, M: Message> std::ops::Deref for Cancel<'ctx, M> {
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
