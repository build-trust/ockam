use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::thread;

use crate::cli;
use crate::worker::Worker;

use ockam_common::commands::ockam_commands::*;
use ockam_message::message::{
    AddressType, Message as OckamMessage, MessageType, Route, RouterAddress,
};
use ockam_router::router::Router;
use ockam_transport::transport::UdpTransport;
use ockam_vault::{types::*, Vault};

pub struct Node;

impl Node {
    pub fn new<V: Vault>(mut vault: V, config: Config) -> Sender<Vec<u8>> {
        // create the input and output channel pairs
        let (tx_in, rx_in) = mpsc::channel::<Vec<u8>>();

        // create router, on which to register addresses, send messages, etc.
        let (router_tx, router_rx) = mpsc::channel();
        let mut router = Router::new(router_rx);

        // in the case of a responder, create a worker which is registered with the router
        // TODO: branch on initiator or responder config, or split into different 'new' fns
        let (worker_tx, worker_rx) = mpsc::channel();
        let worker = Worker::new(router_tx.clone(), worker_tx, worker_rx);

        // create the transport, currently UDP-only, poll it for messages on another thread
        let transport_router_tx = router_tx.clone();
        let (transport_tx, transport_rx) = mpsc::channel();
        if let Ok(mut transport) = UdpTransport::new(
            transport_rx,
            transport_tx,
            transport_router_tx,
            config.local_host.to_string().as_str(),
        ) {
            thread::spawn(move || {
                while router.poll() && transport.poll() && worker.poll() {
                    thread::sleep(std::time::Duration::from_millis(33));
                }
            });
        }

        // let node_router_tx = router_tx.clone();
        let route_config = config.clone();
        let remote_addr = route_config
            .onward_route
            .unwrap_or(Route { addresses: vec![] });

        println!("remotr_addr: {:?}", remote_addr.addresses);
        thread::spawn(move || {
            loop {
                if let Ok(data) = rx_in.recv() {
                    // encrypt data and convert into ockam message
                    let mut msg = OckamMessage::default();
                    msg.message_body = data.clone();
                    msg.message_type = MessageType::Payload;

                    if config.output_to_stdout {
                        // send it to the worker
                        let worker_addr = RouterAddress::worker_router_address_from_str("01242020")
                            .expect("failed to create worker router address");
                        msg.onward_route = Route {
                            addresses: vec![worker_addr],
                        };

                        let cmd = OckamCommand::Router(RouterCommand::SendMessage(msg));
                        router_tx
                            .send(cmd)
                            .expect("failed to send worker message on router");
                    } else {
                        // send it over the transport via the router
                        msg.onward_route = Route {
                            addresses: remote_addr.addresses.clone(),
                        };
                        if let Err(e) =
                            router_tx.send(OckamCommand::Router(RouterCommand::SendMessage(msg)))
                        {
                            eprintln!("error sending to socket: {:?}", e);
                        }
                    }
                } else {
                    eprintln!("fatal error: failed to read from input");
                    std::process::exit(1);
                }
            }
        });

        tx_in
    }
}

#[derive(Clone, Copy)]
pub enum Role {
    Initiator,
    Responder,
}

#[derive(Clone)]
pub struct Config {
    onward_route: Option<Route>,
    output_to_stdout: bool,
    decrypt_output: bool,
    local_host: SocketAddr,
    role: Role,
    vault_path: PathBuf,
}

impl Config {
    pub fn vault_path(&self) -> PathBuf {
        self.vault_path.clone()
    }
}

impl From<cli::Args> for Config {
    fn from(args: cli::Args) -> Self {
        let mut cfg = Config {
            onward_route: None,
            output_to_stdout: false,
            decrypt_output: false,
            local_host: args.local_socket(),
            role: Role::Initiator,
            vault_path: args.vault_path(),
        };

        match args.output_kind() {
            cli::OutputKind::Channel(route) => {
                cfg.onward_route = Some(route);
            }
            cli::OutputKind::Stdout => {
                cfg.output_to_stdout = true;
                cfg.decrypt_output = args.decrypt_output();
            }
        }

        cfg.role = match args.role() {
            cli::ChannelRole::Initiator => Role::Initiator,
            cli::ChannelRole::Responder => Role::Responder,
        };

        cfg
    }
}
