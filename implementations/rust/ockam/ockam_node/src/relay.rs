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

use crate::Context;
use ockam_core::{Encoded, Message, Result, Worker};
use std::marker::PhantomData;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub type RelayMessage = Encoded;

pub struct Relay<W, M>
where
    W: Worker<Context = Context>,
    M: Message,
{
    rx: Receiver<RelayMessage>,
    worker: W,
    ctx: Context,
    _msg: PhantomData<M>,
}

impl<W, M> Relay<W, M>
where
    W: Worker<Context = Context, Message = M>,
    M: Message + Send + 'static,
{
    pub fn new(rx: Receiver<RelayMessage>, worker: W, ctx: Context) -> Self {
        Self {
            rx,
            worker,
            ctx,
            _msg: PhantomData,
        }
    }

    /// A wrapper function around the lifetime of a worker
    async fn run_inner(&mut self) -> Result<()> {
        self.worker.initialize(&mut self.ctx)?;

        // Loop until the last sender disappears
        while let Some(ref enc) = self.rx.recv().await {
            let msg = match M::decode(enc) {
                Ok(msg) => msg,
                _ => continue,
            };

            self.worker.handle_message(&mut self.ctx, msg).await?;
        }

        // Errors that occur during shut-down should be logged, but
        // not re-start the worker automatically!
        match self.worker.shutdown(&mut self.ctx) {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!(
                    "Worker '{}' error during shutdown: {}",
                    self.ctx.address(),
                    e.to_string()
                );
                Ok(())
            }
        }
    }

    /// Run the inner worker and restart it if errors occurs
    async fn run(mut self) {
        while let Err(e) = (&mut self).run_inner().await {
            // todo: replace with tracing::warn!
            eprintln!(
                "Worker '{}' experienced an error and is re-starting: {}",
                self.ctx.address(),
                e.to_string()
            );
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
    let relay = Relay::<W, M>::new(rx, worker, ctx);
    rt.spawn(relay.run());
    tx
}
