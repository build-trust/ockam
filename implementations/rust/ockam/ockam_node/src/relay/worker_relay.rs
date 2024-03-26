use crate::channel_types::SmallReceiver;
use crate::relay::CtrlSignal;
use crate::tokio::runtime::Handle;
use crate::Context;
use cfg_if::cfg_if;
use ockam_core::{Message, RelayMessage, Result, Routed, Worker};
#[cfg(feature = "std")]
use opentelemetry::trace::FutureExt;

/// Worker relay machinery
///
/// Every worker in the Ockam runtime needs a certain amount of logic
/// and state attached to the lifecycle of the user's worker code.
/// The relay manages this state and runtime behaviour.
pub struct WorkerRelay<W> {
    worker: W,
    ctx: Context,
}

impl<W: Worker> WorkerRelay<W> {
    pub fn new(worker: W, ctx: Context) -> Self {
        Self { worker, ctx }
    }
}

impl<W, M> WorkerRelay<W>
where
    W: Worker<Context = Context, Message = M>,
    M: Message + Send + 'static,
{
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
    fn wrap_direct_message(relay_msg: RelayMessage) -> Routed<M> {
        Routed::new(
            relay_msg.destination().clone(),
            relay_msg.source().clone(),
            relay_msg.into_local_message(),
        )
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
        cfg_if! {
            if #[cfg(feature = "std")] {
                let tracing_context = relay_msg.local_message().tracing_context();
                // We set the tracing context retrieved from the local message on the worker context
                // This way, if the worker invokes ctx.send_message() to send a message to another worker,
                // that same tracing context will be passed along when a LocalMessage will be created
                // (see send_from_address_impl)
                self.ctx.set_tracing_context(tracing_context.clone());

                self.worker
                    .handle_message(&mut self.ctx, Self::wrap_direct_message(relay_msg))
                    // make sure we are using the latest tracing context to handle the message
                    // the handle_message future
                    .with_context(tracing_context.update().extract())
                    .await?;
            } else {
                let routed = Self::wrap_direct_message(relay_msg);
                self.worker
                    .handle_message(&mut self.ctx, routed)
                    .await?;
                }
        }

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
                self.shutdown_and_stop_ack().await;
                return;
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
                        // No messages left -- stop now
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

        self.shutdown_and_stop_ack().await;
    }

    async fn shutdown_and_stop_ack(&mut self) {
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
        let relay = WorkerRelay::new(worker, ctx);
        rt.spawn(relay.run(ctrl_rx));
    }
}
