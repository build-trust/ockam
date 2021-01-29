use super::NodeExecutor;
use crate::Address;
use std::any::Any;

/// Implementation of the CreateWorker [`Command`]. Creates and registers a new [`super::Worker`].
pub struct CreateWorkerCommand {
    pub address: Address,
    pub handler: Box<dyn Any + Send>,
}

impl CreateWorkerCommand {
    pub fn run(self, executor: &mut NodeExecutor) -> bool {
        executor.register(self.address, self.handler);
        false
    }
}
