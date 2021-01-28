use crate::Worker;

use super::NodeExecutor;

/// Implementation of the Stop [`Command`]. Stops all [`super::Worker`]s.
#[derive(Clone, Debug)]
pub struct Stop {}

impl Stop {
    /// Stop all workers.
    pub fn run(&self, executor: &mut NodeExecutor) -> bool {
        for worker in executor.registry.values_mut() {
            worker.stopping();
        }
        true
    }
}
