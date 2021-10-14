use crate::{tokio::runtime::Runtime, Context};
use ockam_core::{Processor, Result};

pub(crate) static PROC_ADDR_SUFFIX: &str = "__ockam.internal.proc";

pub struct ProcessorRelay<P>
where
    P: Processor<Context = Context>,
{
    processor: P,
    main: Context,
    aux: Context,
}

impl<P> ProcessorRelay<P>
where
    P: Processor<Context = Context>,
{
    pub fn new(processor: P, main: Context, aux: Context) -> Self {
        Self {
            processor,
            main,
            aux,
        }
    }

    async fn run(self) {
        let mut main = self.main;
        let mut aux = self.aux;
        let mut processor = self.processor;
        let main_addr = main.address();

        match processor.initialize(&mut main).await {
            Ok(()) => {}
            Err(e) => {
                error!(
                    "Failure during '{}' processor initialisation: {}",
                    main.address(),
                    e
                );
            }
        }

        // This future encodes the main processor run loop logic
        let run_loop = async {
            loop {
                let should_continue = processor.process(&mut main).await?;
                if !should_continue {
                    break;
                }
            }

            Result::<()>::Ok(())
        };

        // This future resolves when the mailbox sender is dropped
        let shutdown_signal = async {
            while aux.mailbox_next().await.is_some() {}
            Result::<()>::Ok(())
        };

        // Then select over the two futures
        #[cfg(feature = "std")]
        tokio::select! {
            _ = shutdown_signal => {
                info!("Shutting down processor {}", main_addr);
            },
            _ = run_loop => {}
        };

        // FIXME: implement no_std logic here

        // If we reach this point the router has signalled us to shut down
        match processor.shutdown(&mut main).await {
            Ok(()) => {}
            Err(e) => {
                error!("Failure during '{}' processor shutdown: {}", main_addr, e);
            }
        }
    }

    /// Create a processor relay with two node contexts
    pub(crate) fn init(rt: &Runtime, processor: P, main: Context, aux: Context) {
        let relay = ProcessorRelay::<P>::new(processor, main, aux);
        rt.spawn(relay.run());
    }
}
