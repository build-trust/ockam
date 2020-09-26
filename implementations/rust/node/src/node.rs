use ockam_common::commands::ockam_commands::*;
use ockam_message::message::*;
use ockam_router::router::*;
use ockam_transport::transport::*;
use std::str;
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
                        if let Ok(r) = RouterAddress::channel_router_address_from_str("01020304") {
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
                            message_type: MessageType::Payload,
                            message_body: s.as_bytes().to_vec(),
                        };
                        reply.onward_route.addresses = m.return_route.addresses.clone();
                        if let Ok(r) = RouterAddress::channel_router_address_from_str("01020304") {
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
    let mut channel = TestChannel::new(channel_rx, channel_tx, router_tx_for_channel);

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

fn main() {
    // Create route
    let mut onward_route = Route { addresses: vec![] };
    if let Ok(router_addr_0) = RouterAddress::udp_router_address_from_str("127.0.0.1:4051") {
        onward_route.addresses.push(router_addr_0);
    } else {
        return;
    };

    if let Ok(channel_addr) = RouterAddress::channel_router_address_from_str("01020304") {
        onward_route.addresses.push(channel_addr);
    } else {
        return;
    };

    start_thread("127.0.0.1:4050", onward_route, "Hello Ockam".to_string());

    let mut onward_route = Route { addresses: vec![] };
    if let Ok(router_addr_0) = RouterAddress::udp_router_address_from_str("127.0.0.1:4050") {
        onward_route.addresses.push(router_addr_0);
    } else {
        return;
    };

    if let Ok(channel_addr) = RouterAddress::channel_router_address_from_str("01020304") {
        onward_route.addresses.push(channel_addr);
    } else {
        return;
    };

    start_thread("127.0.0.1:4051", onward_route, "Goodbye Ockam".to_string());

    thread::sleep(time::Duration::from_millis(10000));
}
