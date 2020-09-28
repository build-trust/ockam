#![allow(dead_code)]
use ockam_common::commands::ockam_commands::*;
use ockam_message::message::*;
use ockam_router::router::*;
use ockam_transport::transport::*;
use std::net::SocketAddr;
use std::str;
use std::str::FromStr;
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

pub struct TestWorker {
    rx: std::sync::mpsc::Receiver<OckamCommand>,
    _tx: std::sync::mpsc::Sender<OckamCommand>,
    router_tx: std::sync::mpsc::Sender<OckamCommand>,
    toggle: u16,
}

impl TestWorker {
    pub fn new(
        rx: std::sync::mpsc::Receiver<OckamCommand>,
        tx: std::sync::mpsc::Sender<OckamCommand>,
        router_tx: std::sync::mpsc::Sender<OckamCommand>,
    ) -> Self {
        if let Err(_error) = router_tx.send(OckamCommand::Router(RouterCommand::Register(
            AddressType::Worker,
            tx.clone(),
        ))) {
            println!("send failed in TestChannel::new")
        }
        TestWorker {
            rx,
            _tx: tx,
            router_tx,
            toggle: 0,
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
                    OckamCommand::Worker(WorkerCommand::SendMessage(mut m)) => {
                        // encrypt message body here
                        if let Ok(r) = RouterAddress::worker_router_address_from_str("01020304") {
                            m.return_route.addresses.push(r);
                        } else {
                            return false;
                        }
                        match self
                            .router_tx
                            .send(OckamCommand::Router(RouterCommand::SendMessage(m)))
                        {
                            Ok(()) => {}
                            Err(_unused) => {
                                println!("send to router failed");
                                keep_going = false;
                            }
                        }
                    }
                    OckamCommand::Worker(WorkerCommand::ReceiveMessage(m)) => {
                        // decrypt message body here
                        println!(
                            "Worker received message: {}",
                            str::from_utf8(&m.message_body).unwrap()
                        );
                        let s: &str;
                        if 0 == self.toggle % 2 {
                            s = "Hello Ockam";
                        } else {
                            s = "Goodbye Ockam"
                        };
                        self.toggle += 1;
                        let mut reply: Message = Message {
                            onward_route: Route { addresses: vec![] },
                            return_route: Route { addresses: vec![] },
                            message_type: MessageType::Payload,
                            message_body: s.as_bytes().to_vec(),
                        };
                        reply.onward_route.addresses = m.return_route.addresses.clone();
                        if let Ok(r) = RouterAddress::worker_router_address_from_str("01020304") {
                            reply.return_route.addresses.push(r);
                        } else {
                            return false;
                        }
                        match self
                            .router_tx
                            .send(OckamCommand::Router(RouterCommand::SendMessage(reply)))
                        {
                            Ok(()) => {}
                            Err(_unused) => {
                                println!("send to router failed");
                                keep_going = false;
                            }
                        }
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

pub fn get_route() -> Option<Route> {
    let mut r = Route { addresses: vec![] };
    if let Ok(router_addr_0) = RouterAddress::udp_router_address_from_str("127.0.0.1:4051") {
        r.addresses.push(router_addr_0);
    } else {
        return None;
    };

    if let Ok(channel_addr) = RouterAddress::channel_router_address_from_str("01020304") {
        r.addresses.push(channel_addr);
    } else {
        return None;
    };

    Some(r)
}

fn start_thread(local_address: &str, route: Route, payload: String) {
    let (transport_tx, transport_rx) = std::sync::mpsc::channel();
    let (router_tx, router_rx) = std::sync::mpsc::channel();
    let (channel_tx, channel_rx) = std::sync::mpsc::channel();
    let channel_tx_for_node = channel_tx.clone();
    let router_tx_for_channel = router_tx.clone();

    let mut router = Router::new(router_rx);
    let mut channel = TestWorker::new(channel_rx, channel_tx, router_tx_for_channel);

    let mut transport =
        UdpTransport::new(transport_rx, transport_tx, router_tx, local_address).unwrap();

    let _join_thread: thread::JoinHandle<_> = thread::spawn(move || {
        while transport.poll() && router.poll() & channel.poll() {
            thread::sleep(time::Duration::from_millis(100));
        }
    });

    let m = Message {
        onward_route: route,
        return_route: Route { addresses: vec![] },
        message_type: MessageType::Payload,
        message_body: payload.as_bytes().to_vec(),
    };
    let command = OckamCommand::Channel(ChannelCommand::SendMessage(m));
    match channel_tx_for_node.send(command) {
        Ok(_unused) => {}
        Err(_unused) => {
            println!("failed send to channel");
        }
    }
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

pub fn start_node(local_socket: RouterAddress, onward_route: Route, payload: String) {
    let (transport_tx, transport_rx) = std::sync::mpsc::channel();
    let (router_tx, router_rx) = std::sync::mpsc::channel();
    let (worker_tx, worker_rx) = std::sync::mpsc::channel();
    let worker_tx_for_node = worker_tx.clone();
    let router_tx_for_worker = router_tx.clone();

    let mut router = Router::new(router_rx);
    let mut worker = TestWorker::new(worker_rx, worker_tx, router_tx_for_worker);

    let sock_str: String;
    match local_socket.address {
        Address::UdpAddress(udp) => {
            sock_str = udp.to_string();
            println!("{}", udp.to_string());
        }
        _ => return,
    }

    let mut transport =
        UdpTransport::new(transport_rx, transport_tx, router_tx, &sock_str).unwrap();

    let _join_thread: thread::JoinHandle<_> = thread::spawn(move || {
        while transport.poll() && router.poll() & worker.poll() {
            thread::sleep(time::Duration::from_millis(100));
        }
    });

    if !onward_route.addresses.is_empty() && !payload.is_empty() {
        let m = Message {
            onward_route,
            return_route: Route { addresses: vec![] },
            message_type: MessageType::Payload,
            message_body: payload.as_bytes().to_vec(),
        };
        let command = OckamCommand::Worker(WorkerCommand::SendMessage(m));
        match worker_tx_for_node.send(command) {
            Ok(_unused) => {}
            Err(_unused) => {
                println!("failed send to channel");
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

    start_node(local_socket, route, message);

    thread::sleep(time::Duration::from_millis(1000000));
}
