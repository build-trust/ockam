use crate::relay::CtrlSignal;
use crate::tokio::{runtime::Runtime, sync::mpsc::Receiver};
use crate::{parser, Context};
use core::marker::PhantomData;
use ockam_core::{LocalMessage, Message, Result, Routed, Worker};

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
    fn handle_direct(msg: &LocalMessage) -> Result<M> {
        parser::message::<M>(msg.transport().payload.as_slice()).map_err(|e| {
            error!("Failed to decode message payload for worker" /* FIXME */);
            e
        })
    }

    /// Receive and handle a single message
    ///
    /// Report errors as they occur, and signal whether the loop should
    /// continue running or not
    async fn recv_message(&mut self) -> Result<bool> {
        let relay_msg = match self.ctx.receiver_next().await? {
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
        let msg = Self::handle_direct(&relay_msg.local_msg)?;

        // Wrap the user message in a `Routed` to provide return
        // route information via a composition side-channel
        let routed = Routed::new(msg, relay_msg.addr.clone(), relay_msg.local_msg);

        // Call the worker handle function - pass errors up
        self.worker.handle_message(&mut self.ctx, routed).await?;

        // Signal to the outer loop we would like to run again
        Ok(true)
    }

    #[cfg_attr(not(feature = "std"), allow(unused_mut))]
    #[cfg_attr(not(feature = "std"), allow(unused_variables))]
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

        if let Err(e) = self.ctx.set_ready().await {
            error!("Failed to mark worker '{}' as 'ready': {}", address, e);
        }

        #[cfg(feature = "std")]
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
                        // An error occurred -- log and continue
                        Err(e) => error!("Error encountered during '{}' message handling: {}", address, e),
                    }
                },
                result = ctrl_rx.recv() => {
                    if result.is_some() {
                        debug!("Relay received shutdown signal, terminating!");
                        break;
                    }

                    // We are stopping
                }
            };
        }
        #[cfg(not(feature = "std"))]
        loop {
            match self.recv_message().await {
                // Successful message handling -- keep running
                Ok(true) => {}
                // Successful message handling -- stop now
                Ok(false) => {
                    break;
                }
                // An error occurred -- log and continue
                Err(e) => error!(
                    "Error encountered during '{}' message handling: {}",
                    address, e
                ),
            }
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
            error!("Error occurred during stop ACK sending: {}", e);
        }
    }

    /// Build and spawn a new worker relay, returning a send handle to it
    pub(crate) fn init(rt: &Runtime, worker: W, ctx: Context, ctrl_rx: Receiver<CtrlSignal>) {
        let relay = WorkerRelay::<W, M>::new(worker, ctx);
        rt.spawn(relay.run(ctrl_rx));
    }
}
