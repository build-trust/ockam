use std::sync::mpsc::{self, Receiver, Sender};

use crate::config::Config;

use hex::encode;
use ockam_channel::CHANNEL_ZERO;
use ockam_message::message::{
    Address, AddressType, Codec, Message as OckamMessage, Message, MessageType, Route,
    RouterAddress,
};
use ockam_system::commands::{ChannelCommand, OckamCommand, RouterCommand, WorkerCommand};

pub struct StdinWorker {
    route: Route,
    worker_addr: RouterAddress,
    router_tx: Sender<OckamCommand>,
    rx: Receiver<OckamCommand>,
    tx: Sender<OckamCommand>,
    buf: String,
    config: Config,
    lines_to_send: Vec<String>,
}

impl StdinWorker {
    pub fn initialize(
        config: &Config,
        router_tx: std::sync::mpsc::Sender<OckamCommand>,
        channel_tx: std::sync::mpsc::Sender<OckamCommand>,
    ) -> Option<StdinWorker> {
        let worker = StdinWorker::new(
            RouterAddress::worker_router_address_from_str(&config.service_address().unwrap())
                .expect("failed to create worker address for establishment"),
            router_tx,
            config.clone(),
        );

        // kick off the key exchange process. The result will be that the worker is notified
        // when the secure channel is created.
        let mut onward_route = config.onward_route().unwrap_or(Route { addresses: vec![] });
        onward_route
            .addresses
            .push(RouterAddress::channel_router_address_from_str(CHANNEL_ZERO).unwrap());

        channel_tx
            .send(OckamCommand::Channel(ChannelCommand::Initiate(
                onward_route,
                Address::WorkerAddress(hex::decode(config.service_address().unwrap()).unwrap()),
                None,
            )))
            .unwrap();

        Some(worker)
    }

    pub fn new(
        worker_addr: RouterAddress,
        router_tx: Sender<OckamCommand>,
        config: Config,
    ) -> Self {
        let (tx, rx) = mpsc::channel();

        // register the worker with the router
        router_tx
            .send(OckamCommand::Router(RouterCommand::Register(
                AddressType::Worker,
                tx.clone(),
            )))
            .expect("Stdin worker registration failed");

        Self {
            route: Route { addresses: vec![] },
            worker_addr,
            router_tx,
            rx,
            tx,
            buf: String::new(),
            config,
            lines_to_send: vec![],
        }
    }

    pub fn get_tx(&self) -> Sender<OckamCommand> {
        self.tx.clone()
    }

    fn receive_channel(&mut self, m: Message) -> Result<(), String> {
        self.route = m.return_route.clone();
        self.route.addresses.push(self.worker_addr.clone());

        // add the service address
        let service_address =
            RouterAddress::worker_router_address_from_str(&self.config.service_address().unwrap())
                .unwrap();
        self.route.addresses.push(service_address);

        return match RouterAddress::decode(&m.message_body) {
            Ok((rcc, mb)) => {
                if let Some(rpk) = self.config.public_key_sink() {
                    if rpk == encode(mb) {
                        println!("keys agree");
                        return Ok(());
                    } else {
                        println!("keys conflict");
                        return Err(
                            "remote public key doesn't match expected, possible spoofing".into(),
                        );
                    }
                }
                Ok(())
            }
            _ => Err("receive channel: expected channel address in message body".into()),
        };
    }

    pub fn poll(&mut self) -> bool {
        // await key exchange finalization
        if let Ok(cmd) = self.rx.try_recv() {
            match cmd {
                OckamCommand::Worker(WorkerCommand::ReceiveMessage(msg)) => {
                    match msg.message_type {
                        MessageType::None => match self.receive_channel(msg) {
                            Ok(()) => {}
                            Err(s) => panic!(s),
                        },
                        _ => unimplemented!(),
                    }
                }
                OckamCommand::Worker(WorkerCommand::AddLine(s)) => {
                    self.lines_to_send.push(s);
                }
                _ => unimplemented!(),
            }
        }

        // read from stdin, pass each line to the router within the node
        for s in &self.lines_to_send {
            self.router_tx
                .send(OckamCommand::Router(RouterCommand::SendMessage(
                    OckamMessage {
                        onward_route: self.route.clone(),
                        return_route: Route { addresses: vec![] },
                        message_type: MessageType::Payload,
                        message_body: s.as_bytes().to_vec(),
                    },
                )))
                .expect("failed to send input data to node");
            self.buf.clear();
        }
        self.lines_to_send.clear();
        true
    }
}
