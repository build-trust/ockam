use std::fmt;
use std::fmt::Debug;

use crate::{Address, NodeWorker, WorkerHandle};

use super::NodeExecutor;

/// Implementation of the CreateWorker [`Command`]. Creates and registers a new [`super::Worker`].
#[derive(Clone)]
pub struct CreateWorker {
    pub address: Address,
    pub worker: WorkerHandle,
}

impl Debug for CreateWorker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.address.as_str())
    }
}

impl CreateWorker {
    pub fn run(&self, executor: &mut NodeExecutor) -> bool {
        let context = executor.new_worker_context(self.address.clone());

        let node_worker = NodeWorker::new(context, self.worker.clone());

        executor.register_worker(self.address.clone(), node_worker);

        false
    }
}
