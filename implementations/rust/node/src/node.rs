#![allow(dead_code)]
use ockam_channel::*;
use ockam_common::commands::ockam_commands::*;
use ockam_kex::xx::{XXInitiator, XXResponder};
use ockam_message::message::Address::ChannelAddress;
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

    /// Message - message to send
    #[structopt(short = "m", long = "message")]
    message: Option<String>,
}

pub fn parse_args(args: Args) -> Result<(RouterAddress, Route, String), String> {
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

    if let Some(rs) = args.remote_socket {
        if let Ok(sa) = SocketAddr::from_str(&rs) {
            if let Some(ra) = RouterAddress::from_address(Address::UdpAddress(sa)) {
                route.addresses.push(ra);
            }
        }
    };

    if let Some(wa) = args.worker_addr {
        if let Ok(ra) = RouterAddress::worker_router_address_from_str(&wa) {
            route.addresses.push(ra);
        }
    };

    let mut message = "Hello Ockam".to_string();
    if let Some(m) = args.message {
        message = m;
    };

    Ok((local_socket, route, message))
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
        worker_address: Address,
        mut route: Route,
    ) -> Result<Self, String> {
        if let Err(error) = router_tx.send(OckamCommand::Router(RouterCommand::Register(
            AddressType::Worker,
            tx.clone(),
        ))) {
            return Err("TestWorker failed to register with router".into());
        }
        let mut channel = Address::ChannelAddress(vec![0, 0, 0, 0]); // This address will initiate a key exchange

        Ok(TestWorker {
            rx,
            _tx: tx,
            router_tx,
            address: worker_address,
            channel_address: channel,
            pending_message: None,
            onward_route: route,
            toggle: 0,
        })
    }

    pub fn request_channel(&mut self, mut m: Message) -> Result<(), String> {
        let pending_message = Message {
            onward_route: Route { addresses: vec![] },
            return_route: m.return_route.clone(),
            message_type: MessageType::Payload,
            message_body: m.message_body.clone(),
        };
        self.pending_message = Some(pending_message);
        m.onward_route = self.onward_route.clone();
        m.return_route.addresses.insert(
            0,
            RouterAddress::from_address(self.address.clone()).unwrap(),
        );
        m.message_type = MessageType::None;
        m.message_body = vec![];
        self.router_tx
            .send(OckamCommand::Router(RouterCommand::SendMessage(m)));
        Ok(())
    }

    pub fn handle_send(&mut self, mut m: Message) -> Result<(), String> {
        if self.channel_address.as_string() == "00000000".to_string() {
            // start key exchange
            self.request_channel(m)
        } else {
            m.onward_route = self.onward_route.clone();
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
    // as in the case of a responder, the worker address will be zero and the
    // worker manager should either create a new worker, or bail.
    pub fn receive_channel(&mut self, m: Message) -> Result<(), String> {
        self.channel_address = m.return_route.addresses[0].address.clone();
        let m_opt = self.pending_message.clone();
        match m_opt {
            Some(mut m) => {
                let channel = RouterAddress::from_address(self.channel_address.clone()).unwrap();
                m.onward_route = self.onward_route.clone();
                m.return_route = Route {
                    addresses: vec![RouterAddress::from_address(self.address.clone()).unwrap()],
                };
                self.router_tx
                    .send(OckamCommand::Router(RouterCommand::SendMessage(m)));
                self.pending_message = None;
                return Ok(());
            }
            _ => {
                return Ok(());
            }
        }
        Ok(())
    }

    pub fn handle_receive(&mut self, m: Message) -> Result<(), String> {
        println!("Message type: {}", m.message_type as u16);
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
                let mut reply: Message = Message {
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
                    OckamCommand::Worker(WorkerCommand::SendMessage(mut m)) => {
                        self.handle_send(m).unwrap();
                    }
                    OckamCommand::Worker(WorkerCommand::ReceiveMessage(mut m)) => {
                        match m.message_type {
                            MessageType::Payload => {
                                println!(
                                    "Worker received: {}",
                                    str::from_utf8(&m.message_body).unwrap()
                                );
                            }
                            _ => {}
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
    mut onward_route: Route,
    payload: String,
    is_initiator: bool,
) {
    let (transport_tx, transport_rx) = std::sync::mpsc::channel();
    let (router_tx, router_rx) = std::sync::mpsc::channel();
    let (worker_tx, worker_rx) = std::sync::mpsc::channel();
    let (channel_tx, channel_rx) = std::sync::mpsc::channel();
    let vault = Arc::new(Mutex::new(DefaultVault::default()));

    let mut router = Router::new(router_rx);
    onward_route.addresses.insert(
        0,
        RouterAddress::channel_router_address_from_str("00000000").unwrap(),
    );

    // "onward_route" should consist of:
    // - channel_address 0, which indicates that a secure channel should be established
    // - the list of UDP address hops that comprise the route
    // in this example, the route is defined by command line options
    let mut worker = TestWorker::new(
        worker_rx,
        worker_tx.clone(),
        router_tx.clone(),
        Address::WorkerAddress(hex::decode("00010203").unwrap()), // arbitrary for now
        onward_route,
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
        type XXChannelManager = ChannelManager<XXInitiator, XXResponder, XXInitiator>;
        // the channel handler cannot, in its current implementation, be passed safely
        // between threads, so it is created in the context of the polling thread
        let mut channel_handler =
            XXChannelManager::new(channel_rx, channel_tx.clone(), router_tx.clone(), vault)
                .unwrap();

        while transport.poll() && router.poll() && channel_handler.poll().unwrap() && worker.poll()
        {
            thread::sleep(time::Duration::from_millis(100));
        }
    });

    if is_initiator {
        let onward_route = Route {
            addresses: vec![RouterAddress::worker_router_address_from_str("00010203").unwrap()],
        };
        let m = Message {
            onward_route,
            return_route: Route { addresses: vec![] },
            message_type: MessageType::Payload,
            message_body: payload.as_bytes().to_vec(),
        };
        let command = OckamCommand::Worker(WorkerCommand::SendMessage(m));
        match worker_tx.send(command) {
            Ok(_unused) => {}
            Err(_unused) => {
                println!("failed send to worker");
            }
        }
    }
}

fn main() {
    let args = Args::from_args();
    println!("{:?}", args);
    let local_socket: RouterAddress;
    let route: Route;
    let message: String;
    match parse_args(args) {
        Ok((ls, r, m)) => {
            local_socket = ls;
            route = r;
            message = m;
        }
        Err(s) => {
            println!("{}", s);
            return;
        }
    }
    let is_initiator = !route.addresses.is_empty();

    start_node(local_socket, route, message, is_initiator);

    thread::sleep(time::Duration::from_millis(1000000));
}
