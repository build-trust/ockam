use crate::channel_types::SmallReceiver;
use crate::relay::CtrlSignal;
use crate::tokio::runtime::Handle;
use crate::{parser, Context};
use core::marker::PhantomData;
use ockam_core::{Message, RelayMessage, Result, Routed, Worker};

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

    /// Convenience function to parse an incoming direct message and
    /// wrap it in a [`Routed`]
    ///
    /// This provides return route information for workers via a
    /// composition side-channel.
    ///
    /// This is currently called twice, once when the message is
    /// dispatched to the worker for authorization and again for
    /// handling. Two unpleasant ways to avoid this are:
    ///
    /// 1. Introduce a Sync bound on the Worker trait that allows us
    ///    to pass the message by reference.
    ///
    /// 2. Introduce a Clone bound on the Message trait that allows us
    ///    to perform a cheaper clone on the message.
    ///
    fn wrap_direct_message(relay_msg: RelayMessage) -> Result<Routed<M>> {
        let payload = relay_msg.local_message().transport().payload.as_slice();
        let msg = parser::message::<M>(payload).map_err(|e| {
            error!("Failed to decode message payload for worker" /* FIXME */);
            e
        })?;
        let routed = Routed::new(
            msg,
            relay_msg.destination().clone(),
            relay_msg.into_local_message(),
        );
        Ok(routed)
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

        // Call the worker handle function - pass errors up
        let routed = Self::wrap_direct_message(relay_msg)?;
        self.worker.handle_message(&mut self.ctx, routed).await?;

        // Signal to the outer loop that we would like to run again
        Ok(true)
    }

    #[cfg_attr(not(feature = "std"), allow(unused_mut))]
    #[cfg_attr(not(feature = "std"), allow(unused_variables))]
    async fn run(mut self, mut ctrl_rx: SmallReceiver<CtrlSignal>) {
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
            crate::tokio::select! {
                result = self.recv_message() => {
                    match result {
                        // Successful message handling -- keep running
                        Ok(true) => {},
                        // Successful message handling -- stop now
                        Ok(false) => {
                            break;
                        },
                        // An error occurred -- log and continue
                        Err(e) => {
                            #[cfg(feature = "debugger")]
                            error!("Error encountered during '{}' message handling: {:?}", address, e);
                            #[cfg(not(feature = "debugger"))]
                            error!("Error encountered during '{}' message handling: {}", address, e);
                        }
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
    pub(crate) fn init(rt: &Handle, worker: W, ctx: Context, ctrl_rx: SmallReceiver<CtrlSignal>) {
        let relay = WorkerRelay::<W, M>::new(worker, ctx);
        rt.spawn(relay.run(ctrl_rx));
    }
}
