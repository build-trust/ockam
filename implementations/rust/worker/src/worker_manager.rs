#[allow(unused_imports)]
#[allow(unused_variables)]
#[allow(dead_code)]
use ockam_message::message::{Address, AddressType, Message, MessageType, Receiver, Route, Sender};
use ockam_system::commands::{OckamCommand, RouterCommand};
use std::sync::{Arc, Mutex};

pub struct WorkerManager {
    tx: std::sync::mpsc::Sender<OckamCommand>,
    rx: std::sync::mpsc::Receiver<OckamCommand>,
    router_tx: std::sync::mpsc::Sender<OckamCommand>,
    workers: hashbrown::HashMap<String, Arc<Mutex<dyn Receiver + 'static + Send>>>,
}

impl Sender for WorkerManager {
    fn send(&mut self, _m: Message) -> bool {
        unimplemented!()
    }
}

impl WorkerManager {
    pub fn new(
        tx: std::sync::mpsc::Sender<OckamCommand>,
        rx: std::sync::mpsc::Receiver<OckamCommand>,
        router_tx: std::sync::mpsc::Sender<OckamCommand>,
    ) -> WorkerManager {
        router_tx
            .send(OckamCommand::Router(RouterCommand::Register(
                AddressType::Worker,
                tx.clone(),
            )))
            .unwrap();
        WorkerManager {
            tx,
            rx,
            router_tx,
            workers: hashbrown::HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        a: Address,
        r: Arc<Mutex<dyn Receiver + 'static + Send>>,
    ) -> Result<(), String> {
        self.workers.insert(a.as_string(), r);
        Ok(())
    }

    pub fn poll(&mut self) -> bool {
        true
        //    pub fn poll(wm: Arc<Mutex<WorkerManager>>) -> bool {
        //        let keep_going = true;
        // let mut got = true;
        // while got {
        //     got = false;
        //     if let Ok(c) = self.rx.try_recv() {
        //         got = true;
        //         match c {
        //             // OckamCommand::worker(WorkerCommand::SendMessage(m)) => {
        //             //     self.handle_send(m).unwrap();
        //             // }
        //             // OckamCommand::worker(WorkerCommand::ReceiveMessage(m)) => {
        //             //     if let MessageType::Payload = m.message_type {
        //             //         println!(
        //             //             "worker received: {}",
        //             //             str::from_utf8(&m.message_body).unwrap()
        //             //         );
        //             //     }
        //             //     self.handle_receive(m).unwrap();
        //             // }
        //             // OckamCommand::worker(WorkerCommand::Stop) => {
        //             //     keep_going = false;
        //             // }
        //             _ => println!("worker got bad message"),
        //         }
        //     }
        // }
        //        keep_going
    }
}
