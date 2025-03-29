use crate::channel_types::OneshotReceiver;
use crate::{relay::CtrlSignal, tokio::runtime::Handle, Context};
use ockam_core::{Processor, Result};

pub struct ProcessorRelay<P>
where
    P: Processor<Context = Context>,
{
    processor: P,
    ctx: Context,
}

impl<P> ProcessorRelay<P>
where
    P: Processor<Context = Context>,
{
    pub fn new(processor: P, ctx: Context) -> Self {
        Self { processor, ctx }
    }

    #[cfg_attr(not(feature = "std"), allow(unused_mut))]
    #[cfg_attr(not(feature = "std"), allow(unused_variables))]
    async fn run(self, ctrl_rx: OneshotReceiver<CtrlSignal>) {
        let mut ctx = self.ctx;
        let mut processor = self.processor;

        match processor.initialize(&mut ctx).await {
            Ok(()) => {}
            Err(e) => {
                error!(
                    "Failure during '{}' processor initialisation: {}",
                    ctx.primary_address(),
                    e
                );
                shutdown_and_stop_ack(&mut processor, &mut ctx, false).await;
                return;
            }
        }

        // This future encodes the main processor run loop logic
        let run_loop = async {
            loop {
                match processor.process(&mut ctx).await {
                    Ok(should_continue) => {
                        if !should_continue {
                            break;
                        }
                    }
                    Err(e) => {
                        #[cfg(feature = "debugger")]
                        error!(
                            "Error encountered during '{}' processing: {:?}",
                            ctx.primary_address(),
                            e
                        );
                        #[cfg(not(feature = "debugger"))]
                        error!(
                            "Error encountered during '{}' processing: {}",
                            ctx.primary_address(),
                            e
                        );
                    }
                }
            }

            Result::<()>::Ok(())
        };

        let mut stopped_from_router = false;
        #[cfg(feature = "std")]
        {
            // Select over the two futures
            tokio::select! {
                // This future resolves when a stop control signal is received
                _ = ctrl_rx => {
                    debug!("Shutting down processor {} due to shutdown signal", ctx.primary_address());
                    stopped_from_router = true;
                },
                _ = run_loop => {}
            };
        }

        // TODO wait on run_loop until we have a no_std select! implementation
        #[cfg(not(feature = "std"))]
        match run_loop.await {
            Ok(_) => trace!("Processor shut down cleanly {}", ctx.primary_address()),
            Err(err) => error!("processor run loop aborted with error: {:?}", err),
        };

        // If we reach this point the router has signaled us to shut down
        shutdown_and_stop_ack(&mut processor, &mut ctx, stopped_from_router).await;
    }

    /// Create a processor relay with two node contexts
    pub(crate) fn init(
        rt: &Handle,
        processor: P,
        ctx: Context,
        ctrl_rx: OneshotReceiver<CtrlSignal>,
    ) {
        let relay = ProcessorRelay::<P>::new(processor, ctx);
        rt.spawn(relay.run(ctrl_rx));
    }
}

async fn shutdown_and_stop_ack<P>(processor: &mut P, ctx: &mut Context, stopped_from_router: bool)
where
    P: Processor<Context = Context>,
{
    match processor.shutdown(ctx).await {
        Ok(()) => {}
        Err(e) => {
            error!(
                "Failure during '{}' processor shutdown: {}",
                ctx.primary_address(),
                e
            );
        }
    }

    let router = match ctx.router() {
        Ok(router) => router,
        Err(_) => {
            error!(
                "Failure during '{}' processor shutdown. Can't get router",
                ctx.primary_address()
            );
            return;
        }
    };

    if !stopped_from_router {
        if let Err(e) = router.stop_address(ctx.primary_address(), !stopped_from_router) {
            error!(
                "Failure during '{}' processor shutdown: {}",
                ctx.primary_address(),
                e
            );
        }
    }

    // Finally send the router a stop ACK -- log errors
    trace!("Sending shutdown ACK");
    router.stop_ack(ctx.primary_address()).unwrap_or_else(|e| {
        error!(
            "Failed to send stop ACK for '{}': {}",
            ctx.primary_address(),
            e
        );
    });
}
