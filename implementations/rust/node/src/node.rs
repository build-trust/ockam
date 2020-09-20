use ockam_common::commands::commands::*;
#[allow(unused)]
use ockam_message::message::*;
use ockam_router::router::*;
use ockam_transport::transport::*;
use std::net::IpAddr;
use std::str;
use std::str::FromStr;
use std::thread::sleep;
use std::{thread, time};

pub struct TestChannel {
    rx: std::sync::mpsc::Receiver<OckamCommand>,
    _tx: std::sync::mpsc::Sender<OckamCommand>,
    router_tx: std::sync::mpsc::Sender<OckamCommand>,
}

impl TestChannel {
    pub fn new(
        rx: std::sync::mpsc::Receiver<OckamCommand>,
        tx: std::sync::mpsc::Sender<OckamCommand>,
        router_tx: std::sync::mpsc::Sender<OckamCommand>,
    ) -> Self {
        match router_tx.send(OckamCommand::Router(RouterCommand::Register(
            AddressType::Channel,
            tx.clone(),
        ))) {
            _ => {}
        }
        TestChannel {
            rx,
            _tx: tx,
            router_tx,
        }
    }

    pub fn poll(&mut self) -> bool {
        let mut keep_going = true;
        let mut got = true;
        while got {
            got = false;
            match self.rx.try_recv() {
                Ok(c) => {
                    got = true;
                    match c {
                        OckamCommand::Channel(ChannelCommand::SendMessage(m)) => {
                            // encrypt message body here
                            match self
                                .router_tx
                                .send(OckamCommand::Router(RouterCommand::Route(m)))
                            {
                                Ok(()) => {}
                                Err(_0) => {
                                    println!("send to router failed");
                                    keep_going = false;
                                }
                            }
                        }
                        OckamCommand::Channel(ChannelCommand::ReceiveMessage(m)) => {
                            // decrypt message body here
                            println!(
                                "Channel receive message: {}",
                                str::from_utf8(&m.message_body).unwrap()
                            );
                        }
                        OckamCommand::Channel(ChannelCommand::Stop) => {
                            keep_going = false;
                        }
                        _ => println!("Channel got bad message"),
                    }
                }
                _ => {}
            }
        }
        keep_going
    }
}

pub fn get_route() -> Option<Route> {
    let ip_addr_0 = match IpAddr::from_str("127.0.0.1") {
        Ok(a) => a,
        Err(_0) => return None,
    };
    let udp_addr_0 = Address::UdpAddress(ip_addr_0, 4050);
    let router_addr_0: RouterAddress = match RouterAddress::from_address(udp_addr_0) {
        Some(a) => a,
        None => return None,
    };

    let ip_addr_1 = match IpAddr::from_str("127.0.0.1") {
        Ok(a) => a,
        Err(_0) => return None,
    };
    let udp_addr_1 = Address::UdpAddress(ip_addr_1, 4051);
    let router_addr_1 = match RouterAddress::from_address(udp_addr_1) {
        Some(a) => a,
        None => return None,
    };

    let mut r = Route { addresses: vec![] };
    r.addresses.push(router_addr_0);
    r.addresses.push(router_addr_1);
    Some(r)
}

fn main() {
    // Start transport thread, pass rx ownership
    let (transport_tx, transport_rx) = std::sync::mpsc::channel();
    let (router_tx, router_rx) = std::sync::mpsc::channel();
    let (channel_tx, channel_rx) = std::sync::mpsc::channel();
    let channel_tx_for_node = channel_tx.clone();
    let router_tx_for_channel = router_tx.clone();
    let router_tx_for_transport = router_tx.clone();
    let transport_tx_for_node = transport_tx.clone();

    let mut router = Router::new(router_rx);
    let mut transport = UdpTransport::new(transport_rx, transport_tx, router_tx_for_transport);
    let mut channel = TestChannel::new(channel_rx, channel_tx, router_tx_for_channel);

    let join_thread: thread::JoinHandle<_> = thread::spawn(move || {
        println!("in closure");
        while transport.poll() && router.poll() & channel.poll() {
            thread::sleep(time::Duration::from_millis(100));
        }
    });

    // Establish transport
    let command = TransportCommand::Add("127.0.0.1:4050".to_string(), "127.0.0.1:4051".to_string());
    match transport_tx_for_node.send(OckamCommand::Transport(command)) {
        Ok(_0) => println!("sent socket 1 command to transport"),
        Err(_0) => println!("failed to send command to transport"),
    }

    let command = TransportCommand::Add("127.0.0.1:4051".to_string(), "127.0.0.1:4050".to_string());
    match transport_tx_for_node.send(OckamCommand::Transport(command)) {
        Ok(_0) => println!("sent socket 2 command to transport"),
        Err(_0) => println!("failed to send command to transport"),
    }

    // Create route
    let route = match get_route() {
        Some(r) => r,
        None => {
            println!("get_route failed");
            return;
        }
    };

    let m = Message {
        onward_route: route,
        return_route: Route { addresses: vec![] },
        message_body: "Hello Ockam".to_string().as_bytes().to_vec(),
    };
    let command = OckamCommand::Channel(ChannelCommand::SendMessage(m));
    match channel_tx_for_node.send(command) {
        Ok(_0) => {}
        Err(_0) => {}
    }

    sleep(time::Duration::from_millis(1000));

    let command = TransportCommand::Stop;
    match transport_tx_for_node.send(OckamCommand::Transport(command)) {
        Ok(_0) => println!("sent stop command to transport"),
        Err(_0) => println!("failed to send command to transport"),
    }

    match join_thread.join() {
        Ok(_0) => {}
        Err(_0) => {}
    }
}
