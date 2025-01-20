use crate::channel_types::OneshotReceiver;
use crate::tokio::runtime::Handle;
use crate::{Context, Worker};
use ockam_core::Message;

/// A signal type used to communicate between router and worker relay
#[derive(Clone, Debug)]
pub enum CtrlSignal {
    /// Interrupt current message execution and shut down
    InterruptStop,
}

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
    W: Worker<Message = M>,
    M: Message + Send + 'static,
{
    #[cfg_attr(not(feature = "std"), allow(unused_mut))]
    #[cfg_attr(not(feature = "std"), allow(unused_variables))]
    async fn run(mut self, mut ctrl_rx: OneshotReceiver<CtrlSignal>) {
        match self.worker.initialize(&mut self.ctx).await {
            Ok(()) => {}
            Err(e) => {
                error!(
                    "Failure during '{}' worker initialisation: {}",
                    self.ctx.primary_address(),
                    e
                );
                shutdown_and_stop_ack(&mut self.worker, &mut self.ctx, false).await;
                return;
            }
        }

        #[cfg(feature = "std")]
        loop {
            crate::tokio::select! {
                result = self.worker.process(&mut self.ctx) => {
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
                            error!("Error encountered during '{}' message handling: {:?}", self.ctx.primary_address(), e);
                            #[cfg(not(feature = "debugger"))]
                            error!("Error encountered during '{}' message handling: {}", self.ctx.primary_address(), e);
                        }
                    }
                },
                _ = &mut ctrl_rx => {
                    debug!(primary_address=%self.ctx.primary_address(), "Relay received shutdown signal, terminating!");
                    break;

                    // We are stopping
                }
            };
        }
        #[cfg(not(feature = "std"))]
        loop {
            match self.worker.process(&mut self.ctx).await {
                // Successful message handling -- keep running
                Ok(true) => {}
                // No messages left -- stop now
                Ok(false) => {
                    break;
                }
                // An error occurred -- log and continue
                Err(e) => {
                    #[cfg(feature = "debugger")]
                    error!(
                        "Error encountered during '{}' message handling: {:?}",
                        self.ctx.primary_address(),
                        e
                    );
                    #[cfg(not(feature = "debugger"))]
                    error!(
                        "Error encountered during '{}' message handling: {}",
                        self.ctx.primary_address(),
                        e
                    );
                }
            }
        }

        shutdown_and_stop_ack(&mut self.worker, &mut self.ctx, true).await;
    }

    /// Build and spawn a new worker relay, returning a send handle to it
    pub(crate) fn init(rt: &Handle, worker: W, ctx: Context, ctrl_rx: OneshotReceiver<CtrlSignal>) {
        let relay = WorkerRelay::new(worker, ctx);
        rt.spawn(relay.run(ctrl_rx));
    }
}

async fn shutdown_and_stop_ack<W: Worker>(
    worker: &mut W,
    ctx: &mut Context,
    stopped_from_router: bool,
) {
    // Run the shutdown hook for this worker
    // TODO: pass stopped_from_router to the shutdown, a Worker may choose different strategy on
    //  shutting down dependent workers based on that. E.g., TcpSender should stop TcpReceiver if
    //  we close the TCP connection, but not if we shutdown the node.
    match worker.shutdown(ctx).await {
        Ok(()) => {}
        Err(e) => {
            error!(
                "Failure during '{}' worker shutdown: {}",
                ctx.primary_address(),
                e
            );
        }
    }

    let router = match ctx.router() {
        Ok(router) => router,
        Err(_) => {
            error!(
                "Failure during '{}' worker shutdown. Can't get router",
                ctx.primary_address()
            );
            return;
        }
    };

    if !stopped_from_router {
        if let Err(e) = router.stop_address(ctx.primary_address(), !stopped_from_router) {
            error!(
                "Failure during '{}' worker shutdown: {}",
                ctx.primary_address(),
                e
            );
        }
    }

    // Finally send the router a stop ACK -- log errors
    trace!("Sending shutdown ACK");
    router.stop_ack(ctx.primary_address()).unwrap_or_else(|e| {
        error!(
            "Failed to send stop ACK for worker '{}': {}",
            ctx.primary_address(),
            e
        )
    });
}
