use crate::relay::{CtrlSignal, RelayMessage, RelayPayload};
use crate::tokio::{runtime::Runtime, sync::mpsc::Receiver};
use crate::{parser, Context};
use core::marker::PhantomData;
use ockam_core::compat::vec::Vec;
use ockam_core::{Address, LocalMessage, Message, Result, Route, Routed, TransportMessage, Worker};

/// Worker relay machinery
///
/// Every worker in the Ockam runtime needs a certain amount of logic
/// and state attached to the lifecycle of the user's worker code.
/// The relay manages this state and runtime behaviour.
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
    fn handle_direct(msg: &LocalMessage, msg_addr: Address) -> Result<(M, Route)> {
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
    fn handle_pre_router(msg: &[u8], msg_addr: Address) -> Result<M> {
        M::decode(msg).map_err(|e| {
            error!(
                "Failed to decode wrapped router message for worker {}.  \
             Is your router accepting the correct message type? (ockam_core::RouterMessage)",
                msg_addr
            );
            e
        })
    }

    /// Receive and handle a single message
    ///
    /// Report errors as they occur, and signal whether the loop should
    /// continue running or not
    async fn recv_message(&mut self) -> Result<bool> {
        let RelayMessage { addr, data, .. } = match self.ctx.mailbox_next().await {
            Some(msg) => msg,
            None => {
                trace!("No more messages for worker {}", self.ctx.address());
                return Ok(false);
            }
        };

        // Extract the message type based on the relay message
        // wrap state.  Messages addressed to a router will be of
        // type `RouterMessage`, while generic userspace workers
        // can provide any type they want.
        let (msg, _, local_msg) = (|data| -> Result<(M, Route, LocalMessage)> {
            Ok(match data {
                RelayPayload::Direct(local_msg) => Self::handle_direct(&local_msg, addr.clone())
                    .map(|(msg, r)| (msg, r, local_msg))?,
                RelayPayload::PreRouter(enc_msg, route) => {
                    Self::handle_pre_router(&enc_msg, addr.clone()).map(|m| {
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
        })(data)?;

        // Wrap the user message in a `Routed` to provide return
        // route information via a composition side-channel
        let routed = Routed::new(msg, addr.clone(), local_msg);

        // Call the worker handle function - pass errors up
        self.worker.handle_message(&mut self.ctx, routed).await?;

        // Signal to the outer loop we would like to run again
        Ok(true)
    }

    async fn run(mut self, mut ctrl_rx: Receiver<CtrlSignal>) {
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

        let address = self.ctx.address();

        loop {
            let _ = crate::tokio::select! {
                result = self.recv_message() => {
                    match result {
                        // Successful message handling -- keep running
                        Ok(true) => {},
                        // Successful message handling -- stop now
                        Ok(false) => {
                            break;
                        },
                        // An error occured -- log and continue
                        Err(e) => error!("Error encountered during '{}' message handling: {}", address, e),
                    }
                },
                _ = ctrl_rx.recv() => {
                    debug!("Relay received shutdown signal, terminating!");
                    break;
                }
            };
        }

        // Run the shutdown hook for this worker
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

        // Finally send the router a stop ACK -- log errors
        trace!("Sending shutdown ACK");
        if let Err(e) = self.ctx.send_stop_ack().await {
            error!("Error occured during stop ACK sending: {}", e);
        }
    }

    /// Build and spawn a new worker relay, returning a send handle to it
    pub(crate) fn init(rt: &Runtime, worker: W, ctx: Context, ctrl_rx: Receiver<CtrlSignal>) {
        let relay = WorkerRelay::<W, M>::new(worker, ctx);
        rt.spawn(relay.run(ctrl_rx));
    }
}
