use super::NodeExecutor;

mod create_worker;
use create_worker::*;

mod stop;
use super::Address;
use super::WorkerHandle;
use stop::*;

#[derive(Clone, Debug)]
pub enum Command<T> {
    Stop(Stop),
    CreateWorker(CreateWorker<T>),
}

impl<T> Command<T> {
    pub fn stop() -> Command<T> {
        Command::Stop(Stop {})
    }

    pub fn create_worker(worker: WorkerHandle<T>, address: Address) -> Command<T> {
        Command::CreateWorker(CreateWorker { address, worker })
    }

    pub fn run(&self, executor: &mut NodeExecutor<T>) -> bool {
        match self {
            Command::CreateWorker(command) => command.run(executor),
            Command::Stop(command) => command.run(executor),
        }
    }
}
