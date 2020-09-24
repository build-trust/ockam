use ockam_common::commands::ockam_commands::*;
use ockam_message::*;
use ockam_router::*;
use ockam_transport::*;
use std::str;
use std::thread::sleep;
use std::{thread, time};

pub struct TestChannel {
    rx: std::sync::mpsc::Receiver<OckamCommand>,
    _tx: std::sync::mpsc::Sender<OckamCommand>,
    router_tx: std::sync::mpsc::Sender<OckamCommand>,
    toggle: u16,
}

impl TestChannel {
    pub fn new(
        rx: std::sync::mpsc::Receiver<OckamCommand>,
        tx: std::sync::mpsc::Sender<OckamCommand>,
        router_tx: std::sync::mpsc::Sender<OckamCommand>,
    ) -> Self {
        if let Err(_error) = router_tx.send(OckamCommand::Router(RouterCommand::Register(
            AddressType::Channel,
            tx.clone(),
        ))) {
            println!("send failed in TestChannel::new")
        }
        TestChannel {
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
                    OckamCommand::Channel(ChannelCommand::SendMessage(mut m)) => {
                        // encrypt message body here
                        let address = Address::ChannelAddress(1234);
                        if let Some(r) = RouterAddress::from_address(address) {
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
                    OckamCommand::Channel(ChannelCommand::ReceiveMessage(m)) => {
                        // decrypt message body here
                        println!(
                            "Channel received message: {}",
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
                            message_body: s.as_bytes().to_vec(),
                        };
                        reply.onward_route.addresses = m.return_route.addresses.clone();
                        let address = Address::ChannelAddress(1234);
                        if let Some(r) = RouterAddress::from_address(address) {
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
                    OckamCommand::Channel(ChannelCommand::Stop) => {
                        keep_going = false;
                    }
                    _ => println!("Channel got bad message"),
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

    if let Ok(channel_addr) = RouterAddress::channel_router_address_from_u32(1234) {
        r.addresses.push(channel_addr);
    } else {
        return None;
    }

    Some(r)
}

fn main() {
    // Start transport thread, pass rx ownership
    let (transport_tx, transport_rx) = std::sync::mpsc::channel();
    let (router_tx, router_rx) = std::sync::mpsc::channel();
    let (channel_tx, channel_rx) = std::sync::mpsc::channel();
    let channel_tx_for_node = channel_tx.clone();
    let router_tx_for_channel = router_tx.clone();
    let transport_tx_for_node = transport_tx.clone();

    let mut router = Router::new(router_rx);
    let mut transport = UdpTransport::new(transport_rx, transport_tx, router_tx);
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
        Ok(_unused) => println!("sent socket 1 command to transport"),
        Err(_unused) => println!("failed to send command to transport"),
    }

    let command = TransportCommand::Add("127.0.0.1:4051".to_string(), "127.0.0.1:4050".to_string());
    match transport_tx_for_node.send(OckamCommand::Transport(command)) {
        Ok(_unused) => println!("sent socket 2 command to transport"),
        Err(_unused) => println!("failed to send command to transport"),
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
        Ok(_unused) => {}
        Err(_unused) => {}
    }

    sleep(time::Duration::from_millis(5000));

    let command = TransportCommand::Stop;
    match transport_tx_for_node.send(OckamCommand::Transport(command)) {
        Ok(_unused) => println!("sent stop command to transport"),
        Err(_unused) => println!("failed to send command to transport"),
    }

    match join_thread.join() {
        Ok(_unused) => {}
        Err(_unused) => {}
    }
}
