use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

use crate::cli;
use crate::worker::Worker;

use ockam_channel::*;
use ockam_common::commands::ockam_commands::*;
use ockam_kex::xx::{XXInitiator, XXResponder};
use ockam_message::message::{AddressType, Route, RouterAddress};
use ockam_router::router::Router;
use ockam_transport::transport::UdpTransport;
use ockam_vault::software::DefaultVault;

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
    pub fn new(config: &'a Config) -> (Self, Sender<OckamCommand>) {
        // TODO: temporarily passed into the node, need to re-work
        let (router_tx, router_rx) = std::sync::mpsc::channel();
        let router = Router::new(router_rx);

        // create the vault
        // let vault = Arc::new(Mutex::new(
        //     FilesystemVault::new(config.vault_path()).expect("failed to initialize vault"),
        // ));

        let vault = Arc::new(Mutex::new(DefaultVault::default()));

        // create the channel manager
        type XXChannelManager = ChannelManager<XXInitiator, XXResponder, XXInitiator>;
        let (chan_tx, chan_rx) = mpsc::channel();
        let chan_manager =
            XXChannelManager::new(chan_rx, chan_tx, router_tx.clone(), vault).unwrap();

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
                worker: None,
                router,
                router_tx,
                chan_manager,
                transport_tx: self_transport_tx,
                transport,
            },
            node_router_tx,
        )
    }

    pub fn add_worker(&mut self, worker: Worker) {
        self.router_tx
            .send(OckamCommand::Router(RouterCommand::Register(
                AddressType::Worker,
                worker.sender(),
            )))
            .expect("failed to register worker with router");

        self.worker = Some(worker);
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

#[derive(Debug, Clone, Copy)]
pub enum Input {
    Stdin,
}

#[derive(Debug, Clone)]
pub struct Config {
    onward_route: Option<Route>,
    output_to_stdout: bool,
    local_host: SocketAddr,
    role: Role,
    vault_path: PathBuf,
    input_kind: Input,
}

impl Config {
    pub fn vault_path(&self) -> PathBuf {
        self.vault_path.clone()
    }

    pub fn onward_route(&self) -> Option<Route> {
        self.onward_route.clone()
    }

    pub fn input_kind(&self) -> Input {
        self.input_kind.clone()
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
            input_kind: Input::Stdin,
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

        cfg.input_kind = match args.input_kind() {
            cli::InputKind::Stdin => Input::Stdin,
        };

        cfg
    }
}
