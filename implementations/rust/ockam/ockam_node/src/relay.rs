//! A prototype message relay abstraction
//!
//! The main idea behind this approach is to de-couple the generic
//! workers, messages, and user-code from the executor, and mailbox.
//! The issue around typed user messages in the current approach is
//! that `ockam_node` needs to do type conversions that it doesn't
//! know how to do.
//!
//! This approach introduces two parts: the `Relay<M>`, and the
//! `Switch`.  One is generic, to the specific worker and messages
//! that a user wants to handle.  The connection between the Switch
//! and Relay is non-generic and embeds user messages via encoded
//! payloads.
//!
//! The `Relay` is then responsible for turning the message back into
//! a type and notifying the companion actor.

use crate::{Context, Mailbox};
use ockam_core::{Encoded, Message, Worker};
use std::{marker::PhantomData, sync::Arc};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub type RelayMessage = Encoded;

pub struct Relay<W, M>
where
    W: Worker<Context = Context>,
    M: Message,
{
    worker: W,
    ctx: Context,
    _phantom: PhantomData<M>,
}

impl<W, M> Relay<W, M>
where
    W: Worker<Context = Context, Message = M>,
    M: Message + Send + 'static,
{
    pub fn new(worker: W, ctx: Context) -> Self {
        Self {
            worker,
            ctx,
            _phantom: PhantomData,
        }
    }

    async fn run(mut self) {
        self.worker.initialize(&mut self.ctx).unwrap();

        while let Some(ref enc) = self.ctx.mailbox.next().await {
            let msg = match M::decode(enc) {
                Ok(msg) => msg,
                Err(_) => continue,
            };

            self.worker
                .handle_message(&mut self.ctx, msg)
                .await
                .unwrap();
        }

        self.worker.shutdown(&mut self.ctx).unwrap();
    }

    /// Run the inner worker and restart it if errors occurs
    async fn run_mailbox(mut rx: Receiver<RelayMessage>, mb_tx: Sender<RelayMessage>) {
        // Relay messages into the worker mailbox
        while let Some(enc) = rx.recv().await {
            mb_tx.send(enc).await.unwrap();
        }
    }
}

/// Build and spawn a new worker relay, returning a send handle to it
pub(crate) fn build<W, M>(rt: &Runtime, worker: W, ctx: Context) -> Sender<RelayMessage>
where
    W: Worker<Context = Context, Message = M>,
    M: Message + Send + 'static,
{
    let (tx, rx) = channel(32);
    let mb_tx = ctx.mailbox.sender();
    let relay = Relay::<W, M>::new(worker, ctx);

    rt.spawn(Relay::<W, M>::run_mailbox(rx, mb_tx));
    rt.spawn(relay.run());
    tx
}

/// Build and spawn the root application relay
///
/// The root relay is different from normal worker relays because its
/// message inbox is never automatically run, and instead needs to be
/// polled via a `receive()` call.
pub(crate) fn build_root<W, M>(rt: Arc<Runtime>, mailbox: &Mailbox) -> Sender<RelayMessage>
where
    W: Worker<Context = Context, Message = M>,
    M: Message + Send + 'static,
{
    let (tx, rx) = channel(32);

    let mb_tx = mailbox.sender();
    rt.spawn(Relay::<W, M>::run_mailbox(rx, mb_tx));
    tx
}
