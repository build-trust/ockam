use ockam::secure_channel::CHANNEL_ZERO;
use ockam_message::message::{Address, AddressType, Message, MessageType, Route, RouterAddress};
use ockam_system::commands::{OckamCommand, RouterCommand, WorkerCommand};
use std::str;

pub struct HelloWorker {
    rx: std::sync::mpsc::Receiver<OckamCommand>,
    _tx: std::sync::mpsc::Sender<OckamCommand>,
    router_tx: std::sync::mpsc::Sender<OckamCommand>,
    address: Address,
    channel_address: Address,
    pending_message: Option<Message>,
    onward_route: Route,
    toggle: u16,
}

// todo - let "new" take a channel address to support the case of a new worker for
// an existing channel
impl HelloWorker {
    pub fn new(
        rx: std::sync::mpsc::Receiver<OckamCommand>,
        tx: std::sync::mpsc::Sender<OckamCommand>,
        router_tx: std::sync::mpsc::Sender<OckamCommand>,
        address: Address,
    ) -> Result<Self, String> {
        if router_tx
            .send(OckamCommand::Router(RouterCommand::Register(
                AddressType::Worker,
                tx.clone(),
            )))
            .is_err()
        {
            return Err("TestWorker failed to register with router".into());
        }
        let channel = Address::ChannelAddress(vec![0, 0, 0, 0]); // This address will initiate a key exchange

        Ok(HelloWorker {
            rx,
            _tx: tx,
            router_tx,
            address,
            channel_address: channel,
            pending_message: None,
            onward_route: Route { addresses: vec![] },
            toggle: 0,
        })
    }

    pub fn handle_send(&mut self, mut m: Message) -> Result<(), String> {
        if self.channel_address.as_string() == *CHANNEL_ZERO {
            m.onward_route.addresses.remove(0);
            let pending_message = Message {
                onward_route: m.onward_route.clone(),
                return_route: m.return_route.clone(),
                message_type: MessageType::Payload,
                message_body: m.message_body,
            };
            self.pending_message = Some(pending_message);
            Ok(())
        } else {
            m.onward_route.addresses.insert(
                0,
                RouterAddress::from_address(self.channel_address.clone()).unwrap(),
            );
            m.return_route.addresses.insert(
                0,
                RouterAddress::from_address(self.address.clone()).unwrap(),
            );
            match self
                .router_tx
                .send(OckamCommand::Router(RouterCommand::SendMessage(m)))
            {
                Ok(()) => Ok(()),
                Err(_unused) => Err("handle_send failed in TestWorker".into()),
            }
        }
    }

    // This function is called when a key exchange has been completed and a secure
    // channel created. If it was requested by the worker, as in the case of an
    // initiator, the worker address should be non-zero. If it was not requested,
    // as in the case of a responder, the worker address may be zero, in which case the
    // worker manager should either create a new worker, or bail.
    pub fn receive_channel(&mut self, m: Message) -> Result<(), String> {
        self.channel_address = m.return_route.addresses[0].address.clone();
        self.onward_route
            .addresses
            .push(RouterAddress::worker_router_address_from_str("01020304").unwrap());
        let pending_opt = self.pending_message.clone();
        match pending_opt {
            Some(mut pending) => {
                pending.onward_route.addresses.insert(
                    0,
                    RouterAddress::from_address(self.channel_address.clone()).unwrap(),
                );
                pending.return_route = Route {
                    addresses: vec![RouterAddress::from_address(self.address.clone()).unwrap()],
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

    fn handle_payload(&mut self, _m: Message) -> Result<(), String> {
        let s: &str;
        if 0 == self.toggle % 2 {
            s = "Hello Ockam";
        } else {
            s = "Goodbye Ockam"
        };
        self.toggle += 1;
        let reply: Message = Message {
            onward_route: self.onward_route.clone(),
            return_route: Route {
                addresses: vec![RouterAddress::from_address(self.address.clone()).unwrap()],
            },
            message_type: MessageType::Payload,
            message_body: s.as_bytes().to_vec(),
        };
        match self
            .router_tx
            .send(OckamCommand::Router(RouterCommand::SendMessage(reply)))
        {
            Ok(()) => {}
            Err(_unused) => {
                println!("send to router failed");
                return Err("send to router failed in TestWorker".into());
            }
        }
        Ok(())
    }

    pub fn handle_receive(&mut self, m: Message) -> Result<(), String> {
        self.onward_route = m.return_route.clone(); // next onward_route is always last return_route
        match m.message_type {
            MessageType::Payload => self.handle_payload(m),
            MessageType::None => {
                // MessageType::None indicates new channel
                self.receive_channel(m.clone()).unwrap();
                self.handle_payload(m).unwrap();
                Ok(())
            }
            _ => Err("worker got bad message type".into()),
        }
    }

    pub fn poll(&mut self) -> bool {
        let mut keep_going = true;
        let mut got = true;
        while got {
            got = false;
            if let Ok(c) = self.rx.try_recv() {
                got = true;
                match c {
                    OckamCommand::Worker(WorkerCommand::Test) => {
                        println!("Worker got test command");
                    }
                    OckamCommand::Worker(WorkerCommand::SendMessage(m)) => {
                        self.handle_send(m).unwrap();
                    }
                    OckamCommand::Worker(WorkerCommand::ReceiveMessage(m)) => {
                        if let MessageType::Payload = m.message_type {
                            println!(
                                "Worker received: {}",
                                str::from_utf8(&m.message_body).unwrap()
                            );
                        }
                        self.handle_receive(m).unwrap();
                    }
                    OckamCommand::Worker(WorkerCommand::Stop) => {
                        keep_going = false;
                    }
                    _ => println!("Worker got bad message"),
                }
            }
        }
        keep_going
    }
}
