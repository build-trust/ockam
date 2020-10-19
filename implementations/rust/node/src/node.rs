#![allow(dead_code)]
use ockam_channel::*;
use ockam_common::commands::ockam_commands::*;
use ockam_kex::xx::{XXInitiator, XXNewKeyExchanger, XXResponder};
use ockam_kex::CipherSuite;
use ockam_message::message::*;
use ockam_router::router::*;
use ockam_transport::transport::*;
use ockam_vault::software::DefaultVault;
use std::net::SocketAddr;
use std::str;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::{thread, time};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(author = "Ockam Developers (ockam.io)")]
pub struct Args {
    /// Local address to bind socket
    #[structopt(short = "l", long = "local")]
    local_socket: Option<String>,

    /// Remote address to send to
    #[structopt(short = "r", long = "remote")]
    remote_socket: Option<String>,

    /// Via - intermediate router
    #[structopt(short = "v", long = "via")]
    via_socket: Option<String>,

    /// Worker
    #[structopt(short = "w", long = "worker")]
    worker_addr: Option<String>,
}

pub fn parse_args(args: Args) -> Result<(RouterAddress, Route, RouterAddress), String> {
    let mut local_socket: RouterAddress = RouterAddress {
        a_type: AddressType::Udp,
        length: 7,
        address: (Address::UdpAddress(SocketAddr::from_str("127.0.0.1:4050").unwrap())),
    };
    if let Some(l) = args.local_socket {
        if let Ok(sa) = SocketAddr::from_str(&l) {
            if let Some(ra) = RouterAddress::from_address(Address::UdpAddress(sa)) {
                local_socket = ra;
            }
        }
    } else {
        return Err("local socket address required: -l xxx.xxx.xxx.xxx:pppp".to_string());
    }

    let mut route = Route { addresses: vec![] };

    if let Some(vs) = args.via_socket {
        if let Ok(sa) = SocketAddr::from_str(&vs) {
            if let Some(ra) = RouterAddress::from_address(Address::UdpAddress(sa)) {
                route.addresses.push(ra);
            }
        }
    };

    let mut worker_address = RouterAddress::worker_router_address_from_str("00000000").unwrap();
    if let Some(rs) = args.remote_socket {
        if let Ok(sa) = SocketAddr::from_str(&rs) {
            if let Some(ra) = RouterAddress::from_address(Address::UdpAddress(sa)) {
                route.addresses.push(ra);
                route
                    .addresses
                    .push(RouterAddress::channel_router_address_from_str("00000000").unwrap());
                if let Some(wa) = args.worker_addr {
                    worker_address = RouterAddress::worker_router_address_from_str(&wa).unwrap();
                }
            }
        }
    };

    Ok((local_socket, route, worker_address))
}

pub struct TestWorker {
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
impl TestWorker {
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

        Ok(TestWorker {
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

    // pub fn request_channel(&mut self, mut m: Message) -> Result<(), String> {
    //     let pending_message = Message {
    //         onward_route: m.onward_route.clone(),
    //         return_route: m.return_route.clone(),
    //         message_type: MessageType::Payload,
    //         message_body: m.message_body.clone(),
    //     };
    //     self.pending_message = Some(pending_message);
    //     m.onward_route.addresses.insert(
    //         0,
    //         RouterAddress::channel_router_address_from_str("00000000").unwrap(),
    //     );
    //     m.return_route.addresses.insert(
    //         0,
    //         RouterAddress::from_address(self.address.clone()).unwrap(),
    //     );
    //     m.message_type = MessageType::None;
    //     m.message_body = vec![];
    //     self.router_tx
    //         .send(OckamCommand::Router(RouterCommand::SendMessage(m)))
    //         .unwrap();
    //     Ok(())
    // }

    pub fn handle_send(&mut self, mut m: Message) -> Result<(), String> {
        if self.channel_address.as_string() == *"00000000" {
            m.onward_route.addresses.remove(0);
            let pending_message = Message {
                onward_route: m.onward_route.clone(),
                return_route: m.return_route.clone(),
                message_type: MessageType::Payload,
                message_body: m.message_body.clone(),
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
        let mut return_route = m.return_route.clone();
        self.channel_address = m.return_route.addresses[0].address.clone();
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

    pub fn handle_receive(&mut self, m: Message) -> Result<(), String> {
        self.onward_route = m.return_route.clone(); // next onward_route is always last return_route
        match m.message_type {
            MessageType::Payload => {
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
            MessageType::None => {
                // MessageType::None indicates new channel
                self.receive_channel(m)
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

pub fn start_node(
    local_socket: RouterAddress,
    network_route: Route,
    worker_address: RouterAddress,
    is_initiator: bool,
) {
    let (transport_tx, transport_rx) = std::sync::mpsc::channel();
    let (router_tx, router_rx) = std::sync::mpsc::channel();
    let (worker_tx, worker_rx) = std::sync::mpsc::channel();
    let (channel_tx, channel_rx) = std::sync::mpsc::channel();

    let mut router = Router::new(router_rx);

    let mut worker = TestWorker::new(
        worker_rx,
        worker_tx.clone(),
        router_tx.clone(),
        Address::WorkerAddress(hex::decode("00010203").unwrap()), // arbitrary for now
    )
    .unwrap();

    let sock_str: String;

    match local_socket.address {
        Address::UdpAddress(udp) => {
            sock_str = udp.to_string();
            println!("{}", udp.to_string());
        }
        _ => return,
    }

    let mut transport =
        UdpTransport::new(transport_rx, transport_tx, router_tx.clone(), &sock_str).unwrap();

    let _join_thread: thread::JoinHandle<_> = thread::spawn(move || {
        type XXChannelManager = ChannelManager<XXInitiator, XXResponder, XXNewKeyExchanger>;
        let vault = Arc::new(Mutex::new(DefaultVault::default()));
        let new_key_exchanger = XXNewKeyExchanger::new(
            CipherSuite::Curve25519AesGcmSha256,
            vault.clone(),
            vault.clone(),
        );

        // the channel handler cannot, in its current implementation, be passed safely
        // between threads, so it is created in the context of the polling thread
        let mut channel_handler = XXChannelManager::new(
            channel_rx,
            channel_tx.clone(),
            router_tx.clone(),
            vault,
            new_key_exchanger,
        )
        .unwrap();

        if is_initiator {
            channel_tx.send(OckamCommand::Channel(ChannelCommand::Initiate(
                network_route,
                Address::WorkerAddress(hex::decode("00010203").unwrap()),
            )));
        }

        while transport.poll() && router.poll() && channel_handler.poll().unwrap() && worker.poll()
        {
            thread::sleep(time::Duration::from_millis(100));
        }
    });

    if is_initiator {
        let onward_route = Route {
            addresses: vec![
                RouterAddress::worker_router_address_from_str("00010203").unwrap(),
                worker_address,
            ],
        };
        let m = Message {
            onward_route,
            return_route: Route { addresses: vec![] },
            message_type: MessageType::Payload,
            message_body: "Ping".as_bytes().to_vec(),
        };
        worker_tx.send(OckamCommand::Worker(WorkerCommand::SendMessage(m)));
    }
}

fn main() {
    let args = Args::from_args();
    println!("{:?}", args);
    let local_socket: RouterAddress;
    let route: Route;
    let worker: RouterAddress;
    match parse_args(args) {
        Ok((ls, r, w)) => {
            local_socket = ls;
            route = r;
            worker = w;
        }
        Err(s) => {
            println!("{}", s);
            return;
        }
    }
    let is_initiator = !route.addresses.is_empty();

    println!("route: {:?}", route);

    start_node(local_socket, route, worker, is_initiator);

    thread::sleep(time::Duration::from_millis(1000000));
}
