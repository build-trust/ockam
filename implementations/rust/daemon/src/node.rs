use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

use crate::cli;
use crate::config::{Config, Role};
use crate::sink::SinkWorker;
use crate::source::StdinWorker;

// pub enum OckamdWorker {
//     ockam_daemon::initiator::StdinWorker
// }

use ockam::secure_channel::*;
use ockam_kex::{
    xx::{XXInitiator, XXNewKeyExchanger, XXResponder},
    CipherSuite,
};
use ockam_message::message::{Address, RouterAddress};
use ockam_router::router::Router;
use ockam_system::commands::{OckamCommand, WorkerCommand};
use ockam_transport::tcp::TcpManager;
use ockam_vault_file::ockam_vault::types::*;
use ockam_vault_file::ockam_vault::*;
use ockam_vault_file::FilesystemVault;
use std::net::SocketAddr;
use std::ops::Deref;
use std::str::FromStr;

pub enum OckamdWorker {
    StdinWorker(StdinWorker),
    Sink(SinkWorker),
}

#[allow(dead_code)]
pub struct Node<'a> {
    config: &'a Config,
    chan_manager: ChannelManager<XXInitiator, XXResponder, XXNewKeyExchanger>,
    worker: Option<OckamdWorker>,
    router: Router,
    router_tx: Sender<OckamCommand>,
    transport: TcpManager,
    transport_tx: Sender<OckamCommand>,
    pub channel_tx: Sender<OckamCommand>,
}

pub fn get_console_line(wtx: Sender<OckamCommand>) {
    let mut buf: String = "".into();
    while buf != "q" {
        if std::io::stdin().read_line(&mut buf).is_ok() {
            wtx.send(OckamCommand::Worker(WorkerCommand::AddLine(buf.clone())))
                .unwrap();
        }
        buf.clear();
    }
}

impl<'a> Node<'a> {
    pub fn create_transport(
        config: &Config,
        router_tx: Sender<OckamCommand>,
    ) -> Result<(TcpManager, Sender<OckamCommand>), String> {
        // create the transport, currently TCP-only
        // if role == Router, give it a listen address
        let mut listen_addr: Option<SocketAddr> = None;

        match config.role() {
            Role::Router => {
                let la = config
                    .route_hub()
                    .expect("role requires local IP address for tcp listen");
                listen_addr = Some(la);
            }
            Role::Sink => {
                let la = config.local_socket();
                listen_addr = Some(la);
            }
            _ => {}
        }

        let (transport_tx, transport_rx) = mpsc::channel();
        let mut transport = TcpManager::new(
            transport_rx,
            transport_tx.clone(),
            router_tx,
            listen_addr,
            None,
        )
        .expect("failed to create tcp transport manager");

        // connect to router or sink
        if matches!(config.role(), Role::Source)
            || (matches!(config.role(), Role::Sink) && config.route_hub().is_some())
        {
            let hop = if matches!(config.role(), Role::Source) {
                config.onward_route().unwrap().addresses[0].clone()
            } else {
                let a = Address::TcpAddress(config.route_hub().unwrap());
                RouterAddress::from_address(a).unwrap()
            };
            let sock_addr = SocketAddr::from_str(&hop.address.as_string()).unwrap();
            match transport.connect(sock_addr) {
                Ok(h) => h,
                Err(_) => {
                    panic!("failed to connect, is server running?");
                }
            };
        }
        Ok((transport, transport_tx))
    }

    pub fn new(config: &'a Config) -> Result<Self, String> {
        // TODO: temporarily passed into the node, need to re-work
        let (router_tx, router_rx) = std::sync::mpsc::channel();
        let router = Router::new(router_rx);

        // create the vault, using the FILESYSTEM implementation
        let mut vault =
            FilesystemVault::new(config.vault_path()).expect("failed to initialize vault");

        // check for re-use of provided identity name from CLI args, if not in on-disk in vault
        // generate a new one to be used

        let resp_key_ctx = match vault.get_persistent_secret(&config.identity_name()) {
            Ok(secret) => Some(Arc::new(secret)),
            Err(_) => {
                // if responder, generate keypair and display static public key
                if matches!(config.role(), Role::Sink) || matches!(config.role(), Role::Router) {
                    let attributes = SecretAttributes {
                        stype: SecretType::Curve25519,
                        persistence: SecretPersistence::Persistent,
                        length: CURVE25519_SECRET_LENGTH,
                    };
                    Some(Arc::new(
                        vault
                            .secret_generate(attributes)
                            .expect("failed to generate secret"),
                    ))
                } else {
                    None
                }
            }
        };

        if matches!(config.role(), Role::Sink) && resp_key_ctx.is_some() {
            if let Ok(resp_key) = vault.secret_public_key_get(resp_key_ctx.as_ref().unwrap()) {
                println!("Responder public key: {}", hex::encode(resp_key.as_ref()));
            }
        }

        if matches!(config.role(), Role::Router) && resp_key_ctx.is_some() {
            if let Ok(resp_key) = vault.secret_public_key_get(resp_key_ctx.as_ref().unwrap()) {
                println!("Router public key: {}", hex::encode(resp_key.as_ref()));
            }
        }

        // prepare the vault for use in key exchanger and channel manager
        let vault = Arc::new(Mutex::new(vault));

        // create the channel manager
        type XXChannelManager = ChannelManager<XXInitiator, XXResponder, XXNewKeyExchanger>;
        let (channel_tx, channel_rx) = mpsc::channel();
        let new_key_exchanger = XXNewKeyExchanger::new(
            CipherSuite::Curve25519AesGcmSha256,
            vault.clone(),
            vault.clone(),
        );

        let chan_manager = XXChannelManager::new(
            channel_rx,
            channel_tx.clone(),
            router_tx.clone(),
            vault,
            new_key_exchanger,
            resp_key_ctx,
            None,
        )
        .unwrap();

        if let Ok((transport, transport_tx)) = Node::create_transport(&config, router_tx.clone()) {
            // create the worker
            let mut worker: Option<OckamdWorker> = None;
            if matches!(config.role(), Role::Source) {
                worker = Some(OckamdWorker::StdinWorker(
                    StdinWorker::initialize(config, router_tx.clone(), channel_tx.clone()).unwrap(),
                ));
            }
            if matches!(config.role(), Role::Sink) {
                let worker_addr =
                    RouterAddress::worker_router_address_from_str("01242020").unwrap();
                worker = Some(OckamdWorker::Sink(
                    SinkWorker::initialize(
                        &config,
                        worker_addr,
                        router_tx.clone(),
                        channel_tx.clone(),
                    )
                    .unwrap(),
                ));
            }
            Ok(Self {
                config,
                worker,
                router,
                router_tx,
                chan_manager,
                transport_tx,
                transport,
                channel_tx,
            })
        } else {
            Err("failed to create transport".into())
        }
    }

    pub fn run(mut self) {
        match self.worker {
            Some(worker) => match worker {
                OckamdWorker::Sink(mut w) => {
                    while self.router.poll()
                        && self.transport.poll()
                        && w.poll()
                        && self
                            .chan_manager
                            .poll()
                            .expect("channel manager poll failure")
                    {
                        thread::sleep(time::Duration::from_millis(1));
                    }
                }
                OckamdWorker::StdinWorker(mut w) => {
                    let worker_tx = w.get_tx();
                    thread::spawn(move || get_console_line(worker_tx));
                    while self.router.poll()
                        && self.transport.poll()
                        && w.poll()
                        && self
                            .chan_manager
                            .poll()
                            .expect("channel manager poll failure")
                    {
                        thread::sleep(time::Duration::from_millis(1));
                    }
                }
            },
            None => {
                while self.router.poll()
                    && self.transport.poll()
                    && self
                        .chan_manager
                        .poll()
                        .expect("channel manager poll failure")
                {
                    thread::sleep(time::Duration::from_millis(1));
                }
            }
        }
    }
}
