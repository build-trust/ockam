use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

use crate::cli;
use crate::vault::FilesystemVault;
use crate::worker::Worker;

use ockam_channel::*;
use ockam_common::commands::ockam_commands::*;
use ockam_kex::xx::{XXInitiator, XXResponder};
use ockam_message::message::{AddressType, Message as OckamMessage, Route, RouterAddress};
use ockam_router::router::Router;
use ockam_transport::transport::UdpTransport;
use ockam_vault::DynVault;

pub struct Node<'a> {
    config: &'a Config,
    chan_manager: ChannelManager<XXInitiator, XXResponder, XXInitiator>,
    worker: Option<Worker>,
    router: Router,
    router_tx: Sender<OckamCommand>,
    transport: UdpTransport,
    transport_tx: Sender<OckamCommand>,
}

impl<'a> Node<'a> {
    pub fn new(
        worker: Option<Worker>,
        router: Router,
        router_tx: Sender<OckamCommand>,
        chan_manager: ChannelManager<XXInitiator, XXResponder, XXInitiator>,
        config: &'a Config,
    ) -> (Self, Sender<OckamCommand>) {
        // create the transport, currently UDP-only
        let transport_router_tx = router_tx.clone();
        let (transport_tx, transport_rx) = mpsc::channel();
        let self_transport_tx = transport_tx.clone();
        let transport = UdpTransport::new(
            transport_rx,
            transport_tx,
            transport_router_tx,
            config.local_host.to_string().as_str(),
        )
        .expect("failed to create udp transport");

        let node_router_tx = router_tx.clone();
        (
            Self {
                config,
                worker,
                router,
                router_tx,
                chan_manager,
                transport_tx: self_transport_tx,
                transport,
            },
            node_router_tx,
        )
    }

    pub fn worker_address(&self) -> RouterAddress {
        // TODO: expect address to come from config, etc.
        RouterAddress::worker_router_address_from_str("01242020")
            .expect("failed to convert string to worker address")
    }

    pub fn run(mut self) {
        match self.worker {
            Some(worker) => {
                while self.router.poll()
                    && self.transport.poll()
                    && worker.poll()
                    && self
                        .chan_manager
                        .poll()
                        .expect("channel manager poll failure")
                {
                    thread::sleep(time::Duration::from_millis(333));
                }
            }
            None => {
                while self.router.poll()
                    && self.transport.poll()
                    && self
                        .chan_manager
                        .poll()
                        .expect("channel manager poll failure")
                {
                    thread::sleep(time::Duration::from_millis(333));
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Role {
    Initiator,
    Responder,
}

#[derive(Debug, Clone)]
pub struct Config {
    onward_route: Option<Route>,
    output_to_stdout: bool,
    channel_responder_address: Option<RouterAddress>,
    local_host: SocketAddr,
    role: Role,
    vault_path: PathBuf,
    worker_address: Option<RouterAddress>,
}

impl Config {
    pub fn vault_path(&self) -> PathBuf {
        self.vault_path.clone()
    }

    pub fn onward_route(&self) -> Option<Route> {
        // if let Some(worker_addr) = self.worker_address.clone() {
        //     if let Some(mut onward_route) = self.onward_route.clone() {
        //         onward_route.addresses.push(worker_addr.clone());

        //         return Some(onward_route.clone());
        //     }
        // }

        self.onward_route.clone()
    }

    pub fn channel_responder_address(&self) -> Option<RouterAddress> {
        self.channel_responder_address.clone()
    }
}

impl From<cli::Args> for Config {
    fn from(args: cli::Args) -> Self {
        let mut cfg = Config {
            onward_route: None,
            output_to_stdout: false,
            local_host: args.local_socket(),
            channel_responder_address: Some(
                RouterAddress::channel_router_address_from_str(
                    &args
                        .channel_responder_address()
                        .expect("no channel responder address from args"),
                )
                .expect("failed to create channel router addr from string"),
            ),
            role: Role::Initiator,
            vault_path: args.vault_path(),
            worker_address: None,
        };

        match args.output_kind() {
            cli::OutputKind::Channel(route) => {
                cfg.onward_route = Some(route);
            }
            cli::OutputKind::Stdout => {
                cfg.output_to_stdout = true;
            }
        }

        cfg.role = match args.role() {
            cli::ChannelRole::Initiator => Role::Initiator,
            cli::ChannelRole::Responder => Role::Responder,
        };

        if let Some(worker_addr) = args.worker_address() {
            cfg.worker_address = match RouterAddress::worker_router_address_from_str(&worker_addr) {
                Ok(addr) => Some(addr),
                _ => None,
            }
        }

        println!("{:?}", cfg);
        cfg
    }
}
