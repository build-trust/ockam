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
use futures::{
    future::{select, Either},
    pin_mut,
};
use ockam_core::{Address, Encoded, Error, Message, Worker};
use std::{marker::PhantomData, sync::Arc};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub type RelayMessage = Encoded;

enum Incoming {
    Msg(RelayMessage),
    Err(Error),
}

pub struct Relay<W, M>
where
    W: Worker<Context = Context>,
    M: Message,
{
    worker: W,
    ctx: Context,
    sv_address: Option<Address>,
    _phantom: PhantomData<M>,
}

impl<W, M> Relay<W, M>
where
    W: Worker<Context = Context, Message = M>,
    M: Message + Send + 'static,
{
    /// Utility function to optionally notify supervisors
    #[inline]
    fn sv_notify(&self, e: Error) {
        if let Some(ref addr) = self.sv_address {
            self.ctx.send_message(addr.clone(), e).unwrap();
        }
    }

    pub fn new(worker: W, ctx: Context, sv_address: Option<Address>) -> Self {
        Self {
            worker,
            ctx,
            sv_address,
            _phantom: PhantomData,
        }
    }

    async fn get_next(&mut self, sv_rx: &mut Receiver<Error>) -> Option<Incoming> {
        let sv = sv_rx.recv();
        let mb = self.ctx.mailbox.next();

        pin_mut!(sv);
        pin_mut!(mb);

        match select(sv, mb).await {
            Either::Left((Some(err), _)) => Some(Incoming::Err(err)),
            Either::Right((Some(enc), _)) => Some(Incoming::Msg(enc)),
            _ => None,
        }
    }

    async fn run(mut self, mut sv_rx: Receiver<Error>) {
        // Initialise the worker and report any errors
        if let Err(e) = self.worker.initialize(&mut self.ctx) {
            self.sv_notify(e);
        }

        // Accept messages from both the mailbox, and supervisor error
        // back-channel.  Execute this loop whenever either of them
        // has a new message, and exit the loop when messages run out
        while let Some(inc) = self.get_next(&mut sv_rx).await {
            match inc {
                Incoming::Msg(enc) => {
                    let msg = match M::decode(&enc) {
                        Ok(m) => m,
                        Err(_) => continue,
                    };

                    // Notify the supervisor if any errors occured
                    if let Err(e) = self.worker.handle_message(&mut self.ctx, msg) {
                        self.sv_notify(e);
                    }
                }
                Incoming::Err(e) => self.worker.handle_failures(&mut self.ctx, e),
            }
        }

        if let Err(e) = self.worker.shutdown(&mut self.ctx) {
            self.sv_notify(e);
        }
    }

    /// Run the inner worker and restart it if errors occurs
    async fn run_mailbox(
        mut rx: Receiver<RelayMessage>,
        mb_tx: Sender<RelayMessage>,
        sv_tx: Sender<Error>,
    ) {
        // Check every message for a supervisor error and handle them
        // separately.  Other messages can be queued in the mailbox
        while let Some(enc) = rx.recv().await {
            if let Ok(err) = Error::decode(&enc) {
                sv_tx.send(err).await.unwrap();
            } else {
                mb_tx.send(enc).await.unwrap();
            }
        }
    }
}

/// Build and spawn a new worker relay, returning a send handle to it
pub(crate) fn build<W, M>(
    rt: &Runtime,
    worker: W,
    ctx: Context,
    sv_address: Option<Address>,
) -> Sender<RelayMessage>
where
    W: Worker<Context = Context, Message = M>,
    M: Message + Send + 'static,
{
    let (tx, rx) = channel(32);
    let mb_tx = ctx.mailbox.sender();
    let relay = Relay::<W, M>::new(worker, ctx, sv_address);

    let (sv_tx, sv_rx) = channel(32);

    rt.spawn(Relay::<W, M>::run_mailbox(rx, mb_tx, sv_tx));
    rt.spawn(relay.run(sv_rx));
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
    let (sv_tx, _) = channel(1);

    let mb_tx = mailbox.sender();
    rt.spawn(Relay::<W, M>::run_mailbox(rx, mb_tx, sv_tx));
    tx
}
