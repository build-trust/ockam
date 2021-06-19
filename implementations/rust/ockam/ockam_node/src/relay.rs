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

use crate::{parser, Context, Mailbox};
use ockam_core::{
    Address, LocalMessage, Message, Result, Route, Routed, RouterMessage, TransportMessage, Worker,
};
use std::{marker::PhantomData, sync::Arc};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel, Receiver, Sender};

/// A message addressed to a relay
#[derive(Clone, Debug)]
pub struct RelayMessage {
    addr: Address,
    data: RelayPayload,
    onward: Route,
}

impl RelayMessage {
    /// Construct a message addressed to a user worker
    pub fn direct(addr: Address, local_msg: LocalMessage, onward: Route) -> Self {
        Self {
            addr,
            data: RelayPayload::Direct(local_msg),
            onward,
        }
    }

    /// Construct a message addressed to an middleware router
    #[inline]
    pub fn pre_router(addr: Address, local_msg: LocalMessage, onward: Route) -> Self {
        let route = local_msg.transport().return_route.clone();
        let r_msg = RouterMessage::Route(local_msg);
        Self {
            addr,
            data: RelayPayload::PreRouter(r_msg.encode().unwrap(), route),
            onward,
        }
    }

    /// Consume this message into its base components
    #[inline]
    pub fn local_msg(self) -> (Address, LocalMessage) {
        (
            self.addr,
            match self.data {
                RelayPayload::Direct(msg) => msg,
                _ => panic!("Called transport() on invalid RelayMessage type!"),
            },
        )
    }
}

#[derive(Clone, Debug)]
pub enum RelayPayload {
    Direct(LocalMessage),
    PreRouter(Vec<u8>, Route),
}

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

    /// Convenience function to handle an incoming direct message
    #[inline]
    fn handle_direct(&mut self, msg: &LocalMessage, msg_addr: Address) -> Result<(M, Route)> {
        let TransportMessage {
            ref payload,
            ref return_route,
            ..
        } = msg.transport();

        parser::message::<M>(payload)
            .map_err(|e| {
                error!("Failed to decode message payload for worker {}", msg_addr);
                e
            })
            .map(|m| (m, return_route.clone()))
    }

    #[inline]
    fn handle_pre_router(&mut self, msg: &Vec<u8>, msg_addr: Address) -> Result<M> {
        M::decode(msg).map_err(|e| {
            error!(
                "Failed to decode wrapped router message for worker {}.  \
Is your router accepting the correct message type? (ockam_core::RouterMessage)",
                msg_addr
            );
            e
        })
    }

    async fn run(mut self) {
        self.worker.initialize(&mut self.ctx).await.unwrap();

        while let Some(RelayMessage { addr, data, .. }) = self.ctx.mailbox.next().await {
            // Extract the message type based on the relay message
            // wrap state.  Messages addressed to a router will be of
            // type `RouterMessage`, while generic userspace workers
            // can provide any type they want.
            let (msg, _, local_msg) = match (|data| -> Result<(M, Route, LocalMessage)> {
                Ok(match data {
                    RelayPayload::Direct(local_msg) => self
                        .handle_direct(&local_msg, addr.clone())
                        .map(|(msg, r)| (msg, r, local_msg))?,
                    RelayPayload::PreRouter(enc_msg, route) => {
                        self.handle_pre_router(&enc_msg, addr.clone()).map(|m| {
                            (
                                m,
                                route.clone(),
                                LocalMessage::new(
                                    TransportMessage::v1(Route::new(), route, enc_msg),
                                    Vec::new(),
                                ),
                            )
                        })?
                    }
                })
            })(data)
            {
                Ok((msg, route, transport)) => (msg, route, transport),
                Err(_) => continue, // Handler functions must log
            };

            // Wrap the user message in a `Routed` to provide return
            // route information via a composition side-channel
            let routed = Routed::new(msg, addr.clone(), local_msg);

            // Call the worker handle function
            match self.worker.handle_message(&mut self.ctx, routed).await {
                Ok(()) => {}
                Err(e) => {
                    error!("Worker {} error while handling message: {}", addr, e);
                    continue;
                }
            }
        }

        self.worker.shutdown(&mut self.ctx).await.unwrap();
    }

    /// Run the inner worker and restart it if errors occurs
    async fn run_mailbox(mut rx: Receiver<RelayMessage>, mb_tx: Sender<RelayMessage>) {
        // Relay messages into the worker mailbox
        while let Some(enc) = rx.recv().await {
            match mb_tx.send(enc.clone()).await {
                Ok(x) => x,
                Err(_) => panic!("Failed to route message to address '{}'", enc.addr),
            };
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
