use std::net::SocketAddr;
use std::sync::mpsc::{self, Sender};
use std::thread;

use crate::cli;

use ockam_common::commands::ockam_commands::*;
// use ockam_kex::{xx::*, KeyExchanger};
use ockam_message::{Address, Message as OckamMessage, Route, RouterAddress};
// use ockam_router::Router;
use ockam_transport::UdpTransport;
use ockam_vault::Vault;

pub struct Node<V>
where
    V: Vault,
{
    vault: V,
    config: Config,
}

impl<V> Node<V>
where
    V: Vault,
{
    pub fn new(mut vault: V, config: Config) -> (Self, Sender<Vec<u8>>) {
        // create the input and output channel pairs
        let (tx_in, rx_in) = mpsc::channel();

        let (transport_tx, transport_rx) = mpsc::channel();
        let (router_tx, router_rx) = mpsc::channel();
        // let (channel_tx, channel_rx) = mpsc::channel();
        // let channel_tx_for_node = channel_tx.clone();
        // let router_tx_for_channel = router_tx.clone();

        let transport_tx_for_node = transport_tx.clone();
        // let mut router = Router::new(router_rx);

        let node = Self {
            vault,
            config: config.clone(),
        };

        // create the transport, currently UDP-only, using the first configured onward route
        if !node.config.output_to_stdout {
            let mut transport = UdpTransport::new(transport_rx, transport_tx, router_tx);

            thread::spawn(move || {
                while transport.poll() {
                    thread::sleep(std::time::Duration::from_millis(100));
                }
            });

            match node
                .config
                .onward_route
                .clone()
                .expect("no output configured to be used for onward route")
                .addresses
                .first()
                .expect("onward route configured from output has no route addresses")
                .address
            {
                Address::UdpAddress(addr) => {
                    println!(
                        "transport created using: {:?}, {:?}",
                        node.config.local_host.to_string(),
                        addr.to_string()
                    );
                    transport_tx_for_node
                        .send(OckamCommand::Transport(TransportCommand::Add(
                            node.config.local_host.to_string(),
                            addr.to_string(),
                        )))
                        .expect("failed to add local / remote socket addresses to transport")
                }
                _ => panic!("found invalid UDP address, only UDP is supported"),
            }
        }

        thread::spawn(move || {
            loop {
                if let Ok(data) = rx_in.recv() {
                    // encrypt data and convert into ockam message
                    let mut msg = OckamMessage::default();
                    msg.message_body = data;
                    msg.onward_route = Route {
                        addresses: vec![RouterAddress::udp_router_address_from_str(
                            "127.0.0.1:34254",
                        )
                        .unwrap()],
                    };

                    if !config.output_to_stdout {
                        // send it to the transport
                        if let Err(e) = transport_tx_for_node
                            .send(OckamCommand::Transport(TransportCommand::SendMessage(msg)))
                        {
                            println!("error sending to socket: {:?}", e);
                        }
                    } else {
                        // send it to stdout
                        // TODO: remove println! prefix, maybe use stdout() handle directly
                        if config.decrypt_output {
                            println!(
                                "ockam decrypted: {}",
                                String::from_utf8(msg.message_body)
                                    .expect("message body contains invalid UTF-8 sequence")
                            );
                        } else {
                            println!(
                                "ockam encrypted: {}",
                                String::from_utf8(msg.message_body)
                                    .expect("message body contains invalid UTF-8 sequence")
                            );
                        }
                    }
                }
            }
        });
        (node, tx_in)
    }

    pub fn use_transport(&self) -> bool {
        !self.config.output_to_stdout
    }
}

#[derive(Clone)]
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
}

impl From<cli::Args> for Config {
    fn from(args: cli::Args) -> Self {
        let mut cfg = Config {
            onward_route: None,
            output_to_stdout: false,
            decrypt_output: false,
            local_host: args.local_host(),
            role: Role::Initiator,
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
