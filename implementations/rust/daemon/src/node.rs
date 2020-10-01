use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::thread;

use crate::cli;

use ockam_common::commands::ockam_commands::*;
use ockam_message::message::{Message as OckamMessage, MessageType, Route};
use ockam_router::router::Router;
use ockam_transport::transport::UdpTransport;
use ockam_vault::{types::*, Vault};

pub struct Node;

impl Node {
    pub fn new<V: Vault>(mut vault: V, config: Config) -> Sender<Vec<u8>> {
        // TODO: determine best way to check for existing vault key, and if `1` is always
        // the identity for ockamd to use from a persisted filesystem vault.

        // let key = vault.secret_export(SecretKeyContext::Memory(1)).unwrap();
        // println!("key 1: {:?}", key);

        let kex_attrs = SecretKeyAttributes {
            xtype: SecretKeyType::P256,
            persistence: SecretPersistenceType::Persistent,
            purpose: SecretPurposeType::KeyAgreement,
        };
        let _kex_secret_ctx = vault
            .secret_generate(kex_attrs)
            .expect("failed to create secret for key agreement");

        // create the input and output channel pairs
        let (tx_in, rx_in) = mpsc::channel();

        let (transport_tx, transport_rx) = mpsc::channel();
        let (router_tx, router_rx) = mpsc::channel();
        // let (channel_tx, channel_rx) = mpsc::channel();
        // let channel_tx_for_node = channel_tx.clone();
        // let router_tx_for_channel = router_tx.clone();

        let transport_tx_for_node = transport_tx.clone();
        let mut router = Router::new(router_rx);

        // create the transport, currently UDP-only, using the first configured onward route and
        // poll it for messages on another thread
        if let Ok(mut transport) = UdpTransport::new(
            transport_rx,
            transport_tx,
            router_tx,
            config.local_host.to_string().as_str(),
        ) {
            thread::spawn(move || {
                while transport.poll() && router.poll() {
                    thread::sleep(std::time::Duration::from_millis(100));
                }
            });
        }

        let route_config = config.clone();
        let remote_addr = route_config
            .onward_route
            .expect("invalid address provided for output")
            .clone();
        thread::spawn(move || {
            loop {
                if let Ok(data) = rx_in.recv() {
                    // encrypt data and convert into ockam message
                    let mut msg = OckamMessage::default();
                    msg.message_body = data;
                    msg.onward_route = Route {
                        addresses: remote_addr.addresses.clone(),
                    };
                    msg.message_type = MessageType::Payload;

                    if !config.output_to_stdout {
                        // send it to the transport
                        if let Err(e) = transport_tx_for_node
                            .send(OckamCommand::Transport(TransportCommand::SendMessage(msg)))
                        {
                            eprintln!("error sending to socket: {:?}", e);
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
