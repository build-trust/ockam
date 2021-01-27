mod command;
pub use command::*;

use std::future::Future;

use ockam_core::Error;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::Receiver;

#[derive(Debug)]
pub struct NodeExecutor {
    receiver: Receiver<Command>,
    // registry: HashMap<String, WorkerContext>,
}

impl NodeExecutor {
    pub fn new(receiver: Receiver<Command>) -> Self {
        NodeExecutor { receiver }
    }

    pub fn execute<T>(
        &mut self,
        application: impl Future<Output = T> + 'static + Send,
    ) -> Result<(), Error>
    where
        T: Send + 'static,
    {
        let runtime = Runtime::new().unwrap();

        // TODO: turn app into a worker with an address
        runtime.spawn(application);

        runtime.block_on(async move {
            loop {
                if let Some(command) = self.receiver.recv().await {
                    let should_break = command.run(self);
                    if should_break {
                        break;
                    };
                }
            }
        });

        Ok(())
    }
}
