use std::fmt;
use std::fmt::Debug;

use crate::{Address, NodeWorker, WorkerHandle};

use super::NodeExecutor;

#[derive(Clone)]
pub struct CreateWorker<T> {
    pub address: Address,
    pub worker: WorkerHandle<T>,
}

impl<T> Debug for CreateWorker<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.address.as_str())
    }
}

impl<T> CreateWorker<T> {
    pub fn run(&self, executor: &mut NodeExecutor<T>) -> bool {
        let context = executor.new_worker_context(self.address.clone());

        let node_worker = NodeWorker::new(context, self.worker.clone());

        executor.register_worker(self.address.clone(), node_worker);

        false
    }
}
