use crate::Context;
use ockam_core::{Encoded, Message};
use std::fmt::{self, Debug, Display, Formatter};
use tokio::sync::mpsc::{Receiver, Sender};

/// A mailbox for encoded messages
///
/// Message type information can't be exposed at this stage because
/// they need to either be typed in the [`Relay`](crate::Relay) or in
/// the worker's [`Context`](crate::Context).
#[derive(Debug)]
pub struct Mailbox {
    rx: Receiver<Encoded>,
    tx: Sender<Encoded>,
}

impl Mailbox {
    pub fn new(rx: Receiver<Encoded>, tx: Sender<Encoded>) -> Self {
        Self { rx, tx }
    }

    pub fn sender(&self) -> Sender<Encoded> {
        self.tx.clone()
    }

    /// Get the next message from the mailbox
    pub async fn next(&mut self) -> Option<Encoded> {
        self.rx.recv().await
    }

    /// If a message wasn't expected, requeue it
    pub async fn requeue(&self, msg: Encoded) {
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
    ctx: &'ctx Context,
}

impl<'ctx, M: Message> Cancel<'ctx, M> {
    pub(crate) fn new(inner: M, ctx: &'ctx Context) -> Self {
        Self { inner, ctx }
    }

    /// Cancel this message
    pub async fn cancel(self) {
        let ctx = self.ctx;
        let enc = self.inner.encode().unwrap();
        ctx.mailbox.requeue(enc).await;
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
