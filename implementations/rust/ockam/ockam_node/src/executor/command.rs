use super::NodeExecutor;

mod create_worker;
use create_worker::*;

mod stop;
use stop::*;

#[derive(Clone, Debug)]
pub enum Command {
    Stop(Stop),
    CreateWorker(CreateWorker),
}

impl Command {
    pub fn stop() -> Command {
        Command::Stop(Stop {})
    }

    pub fn create_worker() -> Command {
        Command::CreateWorker(CreateWorker {})
    }

    pub fn run(&self, executor: &mut NodeExecutor) -> bool {
        match self {
            Command::CreateWorker(command) => command.run(executor),
            Command::Stop(command) => command.run(executor),
        }
    }
}
