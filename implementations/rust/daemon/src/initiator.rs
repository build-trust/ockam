use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use crate::config::Config;
use crate::node::Node;

use ockam_common::commands::ockam_commands::*;
use ockam_message::message::{
    AddressType, Message as OckamMessage, MessageType, Route, RouterAddress,
};

pub fn run(config: Config) {
    // configure a node
    let node_config = config.clone();
    let (node, router_tx) = Node::new(&node_config);

    thread::spawn(move || {
        let mut worker = StdinWorker::new(
            RouterAddress::worker_router_address_from_str("01242020")
                .expect("failed to create worker address for kex"),
            router_tx.clone(),
            config.clone(),
        );

        while worker.poll() {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    });

    // run the node to poll its various internal components
    node.run();
}

struct StdinWorker {
    onward_route: Route,
    channel: Option<()>,
    worker_addr: RouterAddress,
    router_tx: Sender<OckamCommand>,
    rx: Receiver<OckamCommand>,
    stdin: std::io::Stdin,
    buf: String,
    config: Config,
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
        }
    }

    fn poll(&mut self) -> bool {
        // read from stdin, pass each line to the router within the node
        if self.stdin.read_line(&mut self.buf).is_ok() {
            if self.channel.is_none() {
                // initiate kex with zero address to create new secure channel
                let mut onward_route = self
                    .config
                    .onward_route()
                    .expect("misconfigured onward route");
                onward_route.addresses.insert(
                    0,
                    RouterAddress::channel_router_address_from_str("00000000")
                        .expect("failed to create zero channel address"),
                );
                let return_worker_addr = self.worker_addr.clone();
                let return_route = Route {
                    addresses: vec![return_worker_addr],
                };
                let kex_msg = OckamMessage {
                    message_type: MessageType::None,
                    message_body: vec![],
                    onward_route,
                    return_route,
                };
                self.router_tx
                    .send(OckamCommand::Router(RouterCommand::SendMessage(kex_msg)))
                    .expect("failed to send kex request message to router");

                // await key exchange finalization
                match self.rx.recv() {
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

                                    self.channel = Some(());
                                    self.onward_route = msg.return_route;
                                }
                                _ => unimplemented!(),
                            }
                        }
                        _ => unimplemented!(),
                    },
                    Err(e) => panic!("failed to handle kex response: {:?}", e),
                }
            }
            self.router_tx
                .send(OckamCommand::Router(RouterCommand::SendMessage(
                    OckamMessage {
                        onward_route: self.onward_route.clone(),
                        return_route: Route { addresses: vec![] },
                        message_body: self.buf.as_bytes().to_vec(),
                        message_type: MessageType::Payload,
                    },
                )))
                .expect("failed to send input data to node");
            self.buf.clear();

            return true;
        } else {
            eprintln!("fatal error: failed to read from input");
            return false;
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
