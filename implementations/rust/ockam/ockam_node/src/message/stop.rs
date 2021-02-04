use crate::executor::Executor;

pub struct Stop;

impl Stop {
    /// Stop all workers.
    pub fn handle(self, _executor: &mut Executor) -> bool {
        true
    }
}
