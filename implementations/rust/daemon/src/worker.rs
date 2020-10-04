use std::sync::mpsc::{self, Receiver, Sender};

use ockam_common::commands::ockam_commands::*;
use ockam_message::message::{AddressType, Message as OckamMessage};

pub struct Worker {
    router_tx: Sender<OckamCommand>,
    rx: Receiver<OckamCommand>,
}

impl Worker {
    pub fn new<'a>(
        router_tx: Sender<OckamCommand>,
        tx: Sender<OckamCommand>,
        rx: Receiver<OckamCommand>,
    ) -> Self {
        let cmd = OckamCommand::Router(RouterCommand::Register(AddressType::Worker, tx.clone()));
        router_tx
            .send(cmd)
            .expect("worker failed to send register command to router");

        Worker { router_tx, rx }
    }

    pub fn poll(&self) -> bool {
        match self.rx.try_recv() {
            Ok(cmd) => match cmd {
                // TODO: change worker command variant to recv before testing remote
                OckamCommand::Worker(WorkerCommand::SendMessage(msg)) => {
                    if let Ok(message) = String::from_utf8(msg.message_body) {
                        println!("worker: {}", message);
                    } else {
                        eprintln!("worker-error: bad data");
                    }
                    true
                }
                _ => {
                    eprintln!("unrecognized worker command");
                    false
                }
            },
            Err(_) => true,
        }
    }
}
