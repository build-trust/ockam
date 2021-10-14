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

use crate::relay::{RelayMessage, RelayPayload};
use crate::tokio::runtime::Runtime;
use crate::{parser, Context};
use core::marker::PhantomData;
use ockam_core::compat::vec::Vec;
use ockam_core::{Address, LocalMessage, Message, Result, Route, Routed, TransportMessage, Worker};

pub struct WorkerRelay<W, M>
where
    W: Worker<Context = Context>,
    M: Message,
{
    worker: W,
    ctx: Context,
    _phantom: PhantomData<M>,
}

impl<W, M> WorkerRelay<W, M>
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
    fn handle_pre_router(&mut self, msg: &[u8], msg_addr: Address) -> Result<M> {
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
        match self.worker.initialize(&mut self.ctx).await {
            Ok(()) => {}
            Err(e) => {
                error!(
                    "Failure during '{}' worker initialisation: {}",
                    self.ctx.address(),
                    e
                );
            }
        }

        while let Some(RelayMessage { addr, data, .. }) = self.ctx.mailbox_next().await {
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
                    error!("Failure during {} worker message handling: {}", addr, e);
                    continue;
                }
            }
        }

        match self.worker.shutdown(&mut self.ctx).await {
            Ok(()) => {}
            Err(e) => {
                error!(
                    "Failure during '{}' worker shutdown: {}",
                    self.ctx.address(),
                    e
                );
            }
        }
    }

    /// Build and spawn a new worker relay, returning a send handle to it
    pub(crate) fn init(rt: &Runtime, worker: W, ctx: Context) {
        let relay = WorkerRelay::<W, M>::new(worker, ctx);
        rt.spawn(relay.run());
    }
}
