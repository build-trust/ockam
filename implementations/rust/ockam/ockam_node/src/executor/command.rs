use crate::{Address, Message, NodeExecutor};
use std::any::Any;

mod create_worker;
use create_worker::*;

mod stop;
use stop::*;

mod send;
use send::*;

/// Commands for [`NodeExecutor`].
pub enum Command {
    /// Create a new Worker.
    CreateWorker(CreateWorkerCommand),

    /// Stop a message to a Worker.
    Send(SendCommand),

    /// Stop the executor.
    Stop(StopCommand),
}

impl Command {
    /// Construct [`Command::CreateWorker`] command.
    pub fn create_worker(handler: Box<dyn Any + Send>, address: Address) -> Command {
        Command::CreateWorker(CreateWorkerCommand { address, handler })
    }

    /// Construct a [`Command::Send`] command.
    pub fn send(address: Address, message: Box<dyn Message + Send>) -> Command {
        Command::Send(SendCommand { address, message })
    }

    /// Construct a [`Command::Stop`] command.
    pub fn stop() -> Command {
        Command::Stop(StopCommand {})
    }

    /// Run the [`Command`] on the [`NodeExecutor`].
    pub fn run(self, executor: &mut NodeExecutor) -> bool {
        match self {
            Command::CreateWorker(command) => command.run(executor),
            Command::Send(command) => command.run(executor),
            Command::Stop(command) => command.run(executor),
        }
    }
}
