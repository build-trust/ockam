#![allow(dead_code)]

use crate::blast_worker::BlastWorker;
use crate::hello_worker::HelloWorker;
use ockam_channel::*;
use ockam_kex::xx::{XXInitiator, XXNewKeyExchanger, XXResponder};
use ockam_kex::CipherSuite;
use ockam_message::message::*;
use ockam_message::MAX_MESSAGE_SIZE;
use ockam_router::router::*;
use ockam_system::commands::{ChannelCommand, OckamCommand, RouterCommand, WorkerCommand};
use ockam_transport::tcp::TcpManager;
use ockam_transport::udp::UdpTransport;
use ockam_vault::software::DefaultVault;
use rand;
use rand::random;
use std::io::stdin;
use std::iter::FromIterator;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::{io, str};
use std::{thread, time};

pub enum IpProtocol {
    Tcp,
    Udp,
}

pub enum Role {
    Source,
    Sink,
    Hub,
    Blaster,
    Blastee,
}

pub fn blast(blaster_tx: std::sync::mpsc::Sender<OckamCommand>) {
    thread::sleep(time::Duration::from_millis(100));
    for i in 0..200 {
        if i == 199 {
            println!("199");
        }
        let f = rand::random::<f64>();
        let payload_size = (f * (MAX_MESSAGE_SIZE - 128) as f64) as usize;
        println!("{}: payload size: {}", i, payload_size);
        let buffer = vec!['a' as char; payload_size];
        let buffer = String::from_iter(buffer);
        blaster_tx
            .send(OckamCommand::Worker(WorkerCommand::SendPayload(
                buffer.into(),
            )))
            .expect("SendPayload failed");
        thread::sleep(time::Duration::from_millis(1));
    }
    thread::sleep(time::Duration::from_secs(600));
    println!("exiting thread");
}

pub fn start_node(
    local_udp: Option<SocketAddr>,
    router_addr: Option<SocketAddr>,
    remote_addr: Option<SocketAddr>,
    _worker_addr: Option<Address>,
    listen_addr: Option<SocketAddr>,
    role: Role,
) -> Result<(), String> {
    let (udp_transport_tx, udp_transport_rx) = std::sync::mpsc::channel();
    let (tcp_transport_tx, tcp_transport_rx) = std::sync::mpsc::channel();
    let (router_tx, router_rx) = std::sync::mpsc::channel();
    let (channel_tx, channel_rx) = std::sync::mpsc::channel();
    let (hello_worker_tx, hello_worker_rx) = std::sync::mpsc::channel();
    let (blast_worker_tx, blast_worker_rx) = std::sync::mpsc::channel();

    let mut router = Router::new(router_rx);

    let mut hello_worker = if matches!(role, Role::Sink) || matches!(role, Role::Source) {
        Some(
            HelloWorker::new(
                hello_worker_rx,
                hello_worker_tx.clone(),
                router_tx.clone(),
                Address::WorkerAddress(hex::decode("00010203").unwrap()), // arbitrary for now
            )
            .unwrap(),
        )
    } else {
        None
    };

    let mut blast_worker = if matches!(role, Role::Blaster) {
        {
            let btx = blast_worker_tx.clone();
            thread::spawn(move || blast(btx));
            Some(
                BlastWorker::new(
                    blast_worker_rx,
                    blast_worker_tx.clone(),
                    router_tx.clone(),
                    Address::WorkerAddress(hex::decode("00010203").unwrap()), // arbitrary for now
                )
                .unwrap(),
            )
        }
    } else if matches!(role, Role::Blastee) {
        Some(
            BlastWorker::new(
                blast_worker_rx,
                blast_worker_tx.clone(),
                router_tx.clone(),
                Address::WorkerAddress(hex::decode("00010203").unwrap()), // arbitrary for now
            )
            .unwrap(),
        )
    } else {
        None
    };

    let mut udp_transport = match local_udp {
        Some(udp_socket) => Some(
            UdpTransport::new(
                udp_transport_rx,
                udp_transport_tx,
                router_tx.clone(),
                udp_socket,
            )
            .unwrap(),
        ),
        None => None,
    };

    let mut tcp_manager = TcpManager::new(
        tcp_transport_rx,
        tcp_transport_tx,
        router_tx.clone(),
        listen_addr,
        None,
    )
    .unwrap();

    type XXChannelManager = ChannelManager<XXInitiator, XXResponder, XXNewKeyExchanger>;
    let vault = Arc::new(Mutex::new(DefaultVault::default()));

    let new_key_exchanger = XXNewKeyExchanger::new(
        CipherSuite::Curve25519AesGcmSha256,
        vault.clone(),
        vault.clone(),
    );

    let mut channel_handler = XXChannelManager::new(
        channel_rx,
        channel_tx.clone(),
        router_tx.clone(),
        vault,
        new_key_exchanger,
        None,
        None,
    )
    .unwrap();

    // if initiator, kick off the key exchange process
    if matches!(role, Role::Source) || matches!(role, Role::Blaster) {
        // create tcp connection
        let hop1_addr: SocketAddr;
        if let Some(ra) = router_addr {
            hop1_addr = ra;
        } else if let Some(ra) = remote_addr {
            hop1_addr = ra;
        } else {
            panic!("no route supplied");
        }

        println!("Connecting to {:?}", hop1_addr);
        let hop1 = match tcp_manager.connect(hop1_addr) {
            Ok(h) => h,
            Err(_) => {
                println!("couldn't connect to {:?}", hop1_addr);
                return Err("failed to connect".into());
            }
        };

        let mut channel_route = Route { addresses: vec![] };
        channel_route
            .addresses
            .push(RouterAddress::from_address(hop1).unwrap());
        channel_route
            .addresses
            .push(RouterAddress::channel_router_address_from_str(CHANNEL_ZERO).unwrap());
        channel_tx
            .send(OckamCommand::Channel(ChannelCommand::Initiate(
                channel_route,
                Address::WorkerAddress(hex::decode("00010203").unwrap()),
                None,
            )))
            .unwrap();
    }

    while router.poll() && channel_handler.poll().unwrap() {
        if let Some(w) = hello_worker.as_mut() {
            if !w.poll() {
                break;
            }
        }
        if let Some(b) = blast_worker.as_mut() {
            if !b.poll() {
                break;
            }
        }
        if let Some(u) = udp_transport.as_mut() {
            if !u.poll() {
                break;
            }
        }
        if !tcp_manager.poll() {
            println!("******");
            break;
        }
        thread::sleep(time::Duration::from_millis(1));
    }
    println!("out of poll loop");
    Ok(())
}
