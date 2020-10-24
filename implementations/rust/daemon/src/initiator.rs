use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use hex;

use crate::config::Config;
use crate::node::Node;

use ockam_message::message::{
    Address, AddressType, Message as OckamMessage, Message, MessageType, Route, RouterAddress,
};
use ockam_system::commands::commands::{
    ChannelCommand, OckamCommand, RouterCommand, WorkerCommand,
};

pub fn run(config: Config) {
    // configure a node
    let node_config = config.clone();
    let (node, router_tx) = Node::new(&node_config);

    let mut worker = StdinWorker::new(
        RouterAddress::worker_router_address_from_str("01242020")
            .expect("failed to create worker address for kex"),
        router_tx.clone(),
        config.clone(),
    );

    // kick off the key exchange process. The result will be that the worker is notified
    // when the secure channel is created.
    node.channel_tx
        .send(OckamCommand::Channel(ChannelCommand::Initiate(
            config.onward_route().clone().unwrap(),
            Address::WorkerAddress(hex::decode(config.service_address().unwrap()).unwrap()),
            None,
        )));

    thread::spawn(move || {
        while worker.poll() {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    });

    // run the node to poll its various internal components
    node.run();
}

struct StdinWorker {
    onward_route: Route,
    channel: Option<RouterAddress>,
    worker_addr: RouterAddress,
    router_tx: Sender<OckamCommand>,
    rx: Receiver<OckamCommand>,
    stdin: std::io::Stdin,
    buf: String,
    config: Config,
    pending_message: Option<Message>,
}

impl StdinWorker {
    fn new(worker_addr: RouterAddress, router_tx: Sender<OckamCommand>, config: Config) -> Self {
        let (tx, rx) = mpsc::channel();

        // register the worker with the router
        router_tx
            .send(OckamCommand::Router(RouterCommand::Register(
                AddressType::Worker,
                tx.clone(),
            )))
            .expect("Stdin worker registration failed");

        Self {
            onward_route: Route { addresses: vec![] },
            channel: None,
            worker_addr,
            router_tx,
            rx,
            stdin: std::io::stdin(),
            buf: String::new(),
            config,
            pending_message: None,
        }
    }

    pub fn receive_channel(&mut self, m: Message) -> Result<(), String> {
        let channel = m.return_route.addresses[0].clone();
        self.channel = Some(channel.clone());
        let pending_opt = self.pending_message.clone();
        match pending_opt {
            Some(mut pending) => {
                pending.onward_route.addresses.insert(0, channel);
                pending.return_route = Route {
                    addresses: vec![self.worker_addr.clone()],
                };
                self.router_tx
                    .send(OckamCommand::Router(RouterCommand::SendMessage(pending)))
                    .unwrap();
                self.pending_message = None;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn poll(&mut self) -> bool {
        // await key exchange finalization
        match self.rx.try_recv() {
            Ok(cmd) => match cmd {
                OckamCommand::Worker(WorkerCommand::ReceiveMessage(msg)) => {
                    match msg.message_type {
                        MessageType::None => {
                            // validate the public key matches the one in our config
                            // TODO: revert this comment to validate the responder public
                            // key if let Some(key) =
                            // config.remote_public_key() {
                            //     validate_public_key(&key, msg.message_body)
                            //         .expect("failed to prove responder identity");
                            // }
                            self.receive_channel(msg);
                        }
                        _ => unimplemented!(),
                    }
                }
                _ => unimplemented!(),
            },
            Err(_) => {}
        }

        // read from stdin, pass each line to the router within the node
        if self.stdin.read_line(&mut self.buf).is_ok() {
            return match self.channel.as_ref() {
                Some(channel) => {
                    self.router_tx
                        .send(OckamCommand::Router(RouterCommand::SendMessage(
                            OckamMessage {
                                //onward_route: self.onward_route.clone(),
                                onward_route: Route {
                                    addresses: vec![channel.clone(), self.worker_addr.clone()],
                                },
                                return_route: Route { addresses: vec![] },
                                message_type: MessageType::Payload,
                                message_body: self.buf.as_bytes().to_vec(),
                            },
                        )))
                        .expect("failed to send input data to node");
                    self.buf.clear();
                    true
                }
                None => {
                    self.pending_message = Some(Message {
                        onward_route: Route {
                            addresses: vec![self.worker_addr.clone()],
                        },
                        return_route: Route { addresses: vec![] },
                        message_type: MessageType::Payload,
                        message_body: self.buf.as_bytes().to_vec(),
                    });
                    true
                }
            };
        } else {
            eprintln!("fatal error: failed to read from input");
            false
        }
    }
}

fn validate_public_key(known: &str, remote: Vec<u8>) -> Result<(), String> {
    if known.as_bytes().to_vec() == remote {
        Ok(())
    } else {
        Err("remote public key mismatch".into())
    }
}
