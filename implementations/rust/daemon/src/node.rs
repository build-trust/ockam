use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time;

use crate::cli;
use crate::worker::Worker;

use ockam_common::commands::ockam_commands::*;
use ockam_message::message::{AddressType, Route, RouterAddress};
use ockam_router::router::Router;
use ockam_transport::transport::UdpTransport;
use ockam_vault::Vault;

pub struct Node {
    worker: Option<Worker>,
    router: Router,
    transport: UdpTransport,
}

impl Node {
    pub fn new<V: Vault>(
        mut vault: V,
        worker: Option<Worker>,
        config: &Config,
    ) -> (Self, Sender<OckamCommand>) {
        // create router, on which to register addresses, send messages, etc.
        let (router_tx, router_rx) = mpsc::channel();
        let router = Router::new(router_rx);

        // register the worker with the router
        match &worker {
            Some(worker) => {
                router_tx
                    .send(OckamCommand::Router(RouterCommand::Register(
                        AddressType::Worker,
                        worker.sender(),
                    )))
                    .expect("failed to register worker with router");
            }
            None => {}
        }

        // create the transport, currently UDP-only
        let transport_router_tx = router_tx.clone();
        let (transport_tx, transport_rx) = mpsc::channel();
        let transport = UdpTransport::new(
            transport_rx,
            transport_tx,
            transport_router_tx,
            config.local_host.to_string().as_str(),
        )
        .expect("failed to create udp transport");

        (
            Self {
                worker,
                router,
                transport,
            },
            router_tx.clone(),
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
                while self.router.poll() && self.transport.poll() && worker.poll() {
                    thread::sleep(time::Duration::from_millis(33));
                }
            }
            None => {
                while self.router.poll() && self.transport.poll() {
                    thread::sleep(time::Duration::from_millis(33));
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
        if let Some(worker_addr) = self.worker_address.clone() {
            if let Some(mut onward_route) = self.onward_route.clone() {
                onward_route.addresses.push(worker_addr.clone());

                return Some(onward_route.clone());
            }
        }

        self.onward_route.clone()
    }
}

impl From<cli::Args> for Config {
    fn from(args: cli::Args) -> Self {
        let mut cfg = Config {
            onward_route: None,
            output_to_stdout: false,
            local_host: args.local_socket(),
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
