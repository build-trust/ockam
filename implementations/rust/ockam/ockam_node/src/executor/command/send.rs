use super::NodeExecutor;
use crate::{Address, Message};

/// Implementation of the Stop [`Command`]. Stops all [`super::Worker`]s.
pub struct SendCommand {
    pub address: Address,
    pub message: Box<dyn Message + Send>,
}

impl SendCommand {
    pub fn run(self, executor: &mut NodeExecutor) -> bool {
        executor.send(self.address, self.message);
        false
    }
}
