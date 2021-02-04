use crate::{executor::Executor, Context};
use ockam_core::{Address, Worker};

mod start_worker;
use start_worker::*;

mod stop;
use stop::*;

pub enum Message {
    /// Start a new Worker.
    StartWorker(StartWorker),
    /// Stop the executor.
    Stop(Stop),
}

impl Message {
    /// Construct [`Message::StartWorker`] command.
    pub fn start_worker(address: Address, worker: Box<dyn Worker<Context = Context>>) -> Message {
        Message::StartWorker(StartWorker { address, worker })
    }

    /// Construct a [`Message::Stop`] command.
    pub fn stop() -> Message {
        Message::Stop(Stop {})
    }

    pub async fn handle(executor: &mut Executor) {
        loop {
            if let Some(message) = executor.receive().await {
                let should_break = match message {
                    Message::StartWorker(message) => message.handle(executor),
                    Message::Stop(message) => message.handle(executor),
                };

                if should_break {
                    break;
                }
            }
        }
    }
}
