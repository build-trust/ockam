use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

use ockam_common::commands::ockam_commands::*;
use ockam_message::message::Message as OckamMessage;

type WorkFn = fn(msg: OckamMessage, router_tx: Sender<OckamCommand>);

pub struct Worker {
    router_tx: Option<Sender<OckamCommand>>,
    rx: Receiver<OckamCommand>,
    tx: Sender<OckamCommand>,
    work_fn: WorkFn,
}

impl Worker {
    pub fn new<'a>(work_fn: WorkFn) -> Self {
        let (tx, rx) = mpsc::channel();
        Worker {
            router_tx: None,
            rx,
            tx,
            work_fn,
        }
    }

    pub fn sender(&self) -> Sender<OckamCommand> {
        self.tx.clone()
    }

    pub fn poll(&self) -> bool {
        match self.rx.try_recv() {
            Ok(cmd) => match cmd {
                OckamCommand::Worker(WorkerCommand::ReceiveMessage(msg)) => {
                    if let Some(tx) = self.router_tx.clone() {
                        (self.work_fn)(msg, tx);
                    }
                    true
                }
                _ => {
                    eprintln!("unrecognized worker command: {:?}", cmd);
                    false
                }
            },
            Err(e) => match e {
                TryRecvError::Empty => true,
                _ => {
                    eprintln!("failed to recv worker rx: {:?}", e);
                    false
                }
            },
        }
    }
}

#[test]
fn test_ockamd_worker() {
    let worker = Worker::new(|_, _| {
        println!("this is from the ockamd worker...");
    });

    std::thread::spawn(move || loop {
        worker.poll();
        std::thread::sleep(std::time::Duration::from_millis(500));
        worker
            .tx
            .send(OckamCommand::Worker(WorkerCommand::ReceiveMessage(
                OckamMessage::default(),
            )))
            .unwrap();
        worker.poll();
    });

    std::thread::sleep(std::time::Duration::from_millis(3000));
}
