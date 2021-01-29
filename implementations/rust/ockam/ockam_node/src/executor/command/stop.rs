use super::NodeExecutor;

/// Implementation of the Stop [`Command`]. Stops all [`super::Worker`]s.
pub struct StopCommand;

impl StopCommand {
    /// Stop all workers.
    pub fn run(self, _executor: &mut NodeExecutor) -> bool {
        true
    }
}
