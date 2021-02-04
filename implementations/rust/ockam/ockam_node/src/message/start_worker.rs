use crate::{executor::Executor, Context};
use ockam_core::{Address, Worker};

/// Implementation of the StartWorker [`Message`]. Starts and registers a new Worker.
pub struct StartWorker {
    pub address: Address,
    pub worker: Box<dyn Worker<Context = Context>>,
}

impl StartWorker {
    pub fn handle(self, executor: &mut Executor) -> bool {
        executor.register(self.address, self.worker).unwrap();
        false
    }
}
