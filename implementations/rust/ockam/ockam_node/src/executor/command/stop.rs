use crate::Worker;

use super::NodeExecutor;

#[derive(Clone, Debug)]
pub struct Stop {}

impl Stop {
    /// Stop all workers.
    pub fn run<T>(&self, executor: &mut NodeExecutor<T>) -> bool {
        for worker in executor.registry.values_mut() {
            worker.stopping();
        }
        true
    }
}
