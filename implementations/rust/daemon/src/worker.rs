use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

use ockam_common::commands::ockam_commands::*;
use ockam_message::message::{
    Address, AddressType, Message as OckamMessage, MessageType, Route, RouterAddress,
};

type WorkFn = fn(self_worker: &Worker, msg: OckamMessage);

pub struct Worker {
    router_tx: Sender<OckamCommand>,
    rx: Receiver<OckamCommand>,
    tx: Sender<OckamCommand>,
    address: RouterAddress,
    pending_message: Option<OckamMessage>,
    work_fn: WorkFn,
}

impl Worker {
    pub fn new(address: RouterAddress, router_tx: Sender<OckamCommand>, work_fn: WorkFn) -> Self {
        debug_assert!(matches!(address.a_type, AddressType::Worker));

        let (tx, rx) = mpsc::channel();

        // register the worker with the router
        let cmd = OckamCommand::Router(RouterCommand::Register(AddressType::Worker, tx.clone()));
        router_tx.send(cmd).expect("failed to register worker");

        Worker {
            router_tx,
            rx,
            tx,
            address,
            pending_message: None,
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
                    match msg.message_type {
                        MessageType::Payload => {
                            (self.work_fn)(&self, msg);
                            true
                        }
                        MessageType::None => match String::from_utf8(msg.message_body) {
                            Ok(key) => {
                                println!("Responder public key: {}", hex::encode(key));
                                true
                            }
                            Err(e) => {
                                eprintln!("Invalid public key: {}", e);
                                false
                            }
                        },
                        _ => unimplemented!(),
                    }
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
    let addr = RouterAddress::worker_router_address_from_str("01242020").unwrap();
    let (fake_router_tx, fake_router_rx) = mpsc::channel();
    Worker::new(addr, fake_router_tx, |_, _| {});

    assert!(fake_router_rx.recv().is_ok());
}
