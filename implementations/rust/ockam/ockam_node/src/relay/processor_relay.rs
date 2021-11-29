use crate::tokio::sync::mpsc::Receiver;
use crate::{relay::CtrlSignal, tokio::runtime::Runtime, Context};
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
    async fn run(self, mut ctrl_rx: Receiver<CtrlSignal>) {
        let mut ctx = self.ctx;
        let mut processor = self.processor;
        let ctx_addr = ctx.address();

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

        // This future encodes the main processor run loop logic
        let run_loop = async {
            loop {
                let should_continue = processor.process(&mut ctx).await?;
                if !should_continue {
                    break;
                }
            }

            Result::<()>::Ok(())
        };

        #[cfg(feature = "std")]
        {
            // This future resolves when a stop control signal is received
            let shutdown_signal = async { ctrl_rx.recv().await };

            // Then select over the two futures
            tokio::select! {
                _ = shutdown_signal => {
                    debug!("Shutting down processor {}", ctx_addr);
                },
                _ = run_loop => {}
            };
        }

        // TODO wait on run_loop until we have a no_std select! implementation
        #[cfg(not(feature = "std"))]
        match run_loop.await {
            Ok(_) => trace!("Processor shut down cleanly {}", ctx_addr),
            Err(err) => error!("processor run loop aborted with error: {:?}", err),
        };

        // If we reach this point the router has signalled us to shut down
        match processor.shutdown(&mut ctx).await {
            Ok(()) => {}
            Err(e) => {
                error!("Failure during '{}' processor shutdown: {}", ctx_addr, e);
            }
        }

        // Finally send the router a stop ACK -- log errors
        trace!("Sending shutdown ACK");
        if let Err(e) = ctx.send_stop_ack().await {
            error!("Error occured during stop ACK sending: {}", e);
        }
    }

    /// Create a processor relay with two node contexts
    pub(crate) fn init(rt: &Runtime, processor: P, ctx: Context, ctrl_rx: Receiver<CtrlSignal>) {
        let relay = ProcessorRelay::<P>::new(processor, ctx);
        rt.spawn(relay.run(ctrl_rx));
    }
}
