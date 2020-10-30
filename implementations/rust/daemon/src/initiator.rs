use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use crate::config::Config;
use crate::node::Node;

use crate::cli::ChannelRole::Router;
use hex::encode;
use ockam_channel::CHANNEL_ZERO;
use ockam_message::message::{
    Address, AddressType, Message as OckamMessage, Message, MessageType, Route, RouterAddress,
};
use ockam_system::commands::{ChannelCommand, OckamCommand, RouterCommand, WorkerCommand};

pub fn run(config: Config) {
    // configure a node
    let node_config = config.clone();
    let (node, router_tx) = Node::new(&node_config);

    let mut worker = StdinWorker::new(
        RouterAddress::worker_router_address_from_str(&config.service_address().unwrap())
            .expect("failed to create worker address for kex"),
        router_tx,
        config.clone(),
    );

    // kick off the key exchange process. The result will be that the worker is notified
    // when the secure channel is created.
    println!(
        "Worker address: {:?}",
        hex::decode(config.service_address().unwrap()).unwrap()
    );

    let mut onward_route = Route { addresses: vec![] };
    if let Some(router_address) = config.router_socket() {
        onward_route
            .addresses
            .push(RouterAddress::from_address(Address::UdpAddress(router_address)).unwrap());
    }
    if let Some(channel_to_sink) = config.channel_to_sink() {
        onward_route
            .addresses
            .push(RouterAddress::channel_router_address_from_str(&channel_to_sink).unwrap());
    }
    onward_route
        .addresses
        .push(RouterAddress::channel_router_address_from_str(CHANNEL_ZERO).unwrap());
    println!("Initiating channel with route");
    onward_route.print_route();

    node.channel_tx
        .send(OckamCommand::Channel(ChannelCommand::Initiate(
            onward_route,
            Address::WorkerAddress(hex::decode(config.service_address().unwrap()).unwrap()),
            None,
        )))
        .unwrap();

    thread::spawn(move || {
        while worker.poll() {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    });

    // run the node to poll its various internal components
    node.run();
}

struct StdinWorker {
    //channel: Option<RouterAddress>,
    route: Route,
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
                tx,
            )))
            .expect("Stdin worker registration failed");

        Self {
            route: Route { addresses: vec![] },
            worker_addr,
            router_tx,
            rx,
            stdin: std::io::stdin(),
            buf: String::new(),
            config,
        }
    }

    fn receive_channel(&mut self, m: Message) -> Result<(), String> {
        let channel = m.return_route.addresses[0].clone();
        self.route = m.return_route.clone();

        // add the service address
        let service_address =
            RouterAddress::worker_router_address_from_str(&self.config.service_address().unwrap())
                .unwrap();
        self.route.addresses.push(service_address);

        let resp_public_key = encode(&m.message_body);
        if let Some(rpk) = self.config.remote_public_key() {
            if rpk == encode(&m.message_body) {
                println!("keys agree");
                return Ok(());
            } else {
                println!("keys conflict");
                return Err("remote public key doesn't match expected, possible spoofing".into());
            }
        }
        Ok(())
    }

    fn poll(&mut self) -> bool {
        // await key exchange finalization
        // match self.rx.try_recv() {
        //     Ok(cmd) => match cmd {
        if let Ok(cmd) = self.rx.try_recv() {
            match cmd {
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
                            match self.receive_channel(msg) {
                                Ok(()) => {}
                                Err(s) => panic!(s),
                            }
                        }
                        _ => unimplemented!(),
                    }
                }
                _ => unimplemented!(),
            }
        }

        // read from stdin, pass each line to the router within the node
        if self.route.addresses.len() > 0 {
            return if self.stdin.read_line(&mut self.buf).is_ok() {
                self.router_tx
                    .send(OckamCommand::Router(RouterCommand::SendMessage(
                        OckamMessage {
                            //onward_route: self.onward_route.clone(),
                            onward_route: self.route.clone(),
                            return_route: Route { addresses: vec![] },
                            message_type: MessageType::Payload,
                            message_body: self.buf.as_bytes().to_vec(),
                        },
                    )))
                    .expect("failed to send input data to node");
                self.buf.clear();
                true
            } else {
                println!("failed to read stdin");
                false
            };
        }
        true
    }
}

#[allow(dead_code)]
fn validate_public_key(known: &str, remote: Vec<u8>) -> Result<(), String> {
    if known.as_bytes().to_vec() == remote {
        Ok(())
    } else {
        Err("remote public key mismatch".into())
    }
}
