use create_worker::*;
use stop::*;

use super::Address;
use super::NodeExecutor;
use super::WorkerHandle;

mod create_worker;
mod stop;

/// Commands for [`NodeExecutor`].
#[derive(Clone, Debug)]
pub enum Command {
    /// Stop the system.
    Stop(Stop),

    /// Create a new Worker.
    CreateWorker(CreateWorker),
}

impl Command {
    /// Construct a [`Command::Stop`] command.
    pub fn stop() -> Command {
        Command::Stop(Stop {})
    }

    /// Construct [`Command::CreateWorker`] command.
    pub fn create_worker(worker: WorkerHandle, address: Address) -> Command {
        Command::CreateWorker(CreateWorker { address, worker })
    }

    /// Run the [`Command`] on the [`NodeExecutor`].
    pub fn run(&self, executor: &mut NodeExecutor) -> bool {
        match self {
            Command::CreateWorker(command) => command.run(executor),
            Command::Stop(command) => command.run(executor),
        }
    }
}
