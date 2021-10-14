#[cfg(feature = "std")]
use crate::error::Error;
use crate::relay::{ShutdownHandle, ShutdownListener};
use crate::tokio::runtime::Runtime;
use crate::Context;
use ockam_core::{Processor, Result};

pub struct ProcessorRelay<P>
where
    P: Processor<Context = Context>,
{
    processor: P,
    ctx: Context,
    shutdown_listener: ShutdownListener,
}

impl<P> ProcessorRelay<P>
where
    P: Processor<Context = Context>,
{
    pub fn new(processor: P, ctx: Context, shutdown_listener: ShutdownListener) -> Self {
        Self {
            processor,
            ctx,
            shutdown_listener,
        }
    }

    async fn run(self) {
        let (_rx_shutdown, tx_ack) = self.shutdown_listener.consume();
        let mut ctx = self.ctx;
        let mut processor = self.processor;

        match processor.initialize(&mut ctx).await {
            Ok(()) => {}
            Err(e) => {
                error!(
                    "Failure during '{}' processor initialisation: {}",
                    ctx.address(),
                    e
                );
            }
        }

        let run_loop = async {
            loop {
                let should_continue = processor.process(&mut ctx).await?;
                if !should_continue {
                    break;
                }
            }

            Result::<()>::Ok(())
        };

        #[allow(dead_code)]
        #[derive(Debug)]
        enum StopReason {
            Shutdown,
            LoopStop,
            ProcessError(ockam_core::Error),
            RxError(ockam_core::Error),
        }

        #[cfg(feature = "std")]
        let stop_reason;
        #[cfg(feature = "std")]
        tokio::select! {
            res = _rx_shutdown => {
                match res {
                    Ok(_) => stop_reason = StopReason::Shutdown,
                    Err(_) => stop_reason = StopReason::RxError(Error::ShutdownRxError.into()),
                }
            }
            res = run_loop => {
                match res {
                    Ok(_) => stop_reason = StopReason::LoopStop,
                    Err(err) => stop_reason = StopReason::ProcessError(err),
                }
            }
        }

        // TODO wait on run_loop until we have a no_std select! implementation
        #[cfg(not(feature = "std"))]
        let stop_reason = match run_loop.await {
            Ok(_) => StopReason::LoopStop,
            Err(err) => StopReason::ProcessError(err),
        };

        match processor.shutdown(&mut ctx).await {
            Ok(()) => {}
            Err(e) => {
                error!(
                    "Failure during '{}' processor shutdown: {}",
                    ctx.address(),
                    e
                );
            }
        }

        if tx_ack.send(()).is_err() {
            error!("Failure during shutdown ack '{}'", ctx.address())
        }

        debug!(
            "Stopping processor '{}' with reason {:?}",
            ctx.address(),
            stop_reason
        );

        match stop_reason {
            StopReason::Shutdown => {}
            StopReason::LoopStop => {
                if let Err(err) = ctx.stop_processor(ctx.address()).await {
                    error!("Failure during '{}' processor stop: {}", ctx.address(), err)
                }
            }
            StopReason::ProcessError(err) | StopReason::RxError(err) => {
                error!("Processor '{}' error: {}", ctx.address(), err)
            }
        };
    }

    pub(crate) fn build(rt: &Runtime, processor: P, ctx: Context) -> ShutdownHandle
    where
        P: Processor<Context = Context>,
    {
        let (handle, listener) = ShutdownHandle::create();
        let relay = ProcessorRelay::<P>::new(processor, ctx, listener);

        rt.spawn(relay.run());
        handle
    }
}
