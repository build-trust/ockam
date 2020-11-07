use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

use crate::cli;
use crate::config::{Config, Role};
use crate::initiator::StdinWorker;
use crate::worker::Worker;

// pub enum OckamdWorker {
//     ockam_daemon::initiator::StdinWorker
// }

use ockam_channel::*;
use ockam_kex::{
    xx::{XXInitiator, XXNewKeyExchanger, XXResponder},
    CipherSuite,
};
use ockam_message::message::{Address, AddressType, RouterAddress};
use ockam_router::router::Router;
use ockam_system::commands::{OckamCommand, RouterCommand, WorkerCommand};
use ockam_transport::tcp::{TcpManager, TcpTransport};
use ockam_vault::types::*;
use ockam_vault::{file::FilesystemVault, DynVault};
use std::io::stdin;
use std::net::SocketAddr;
use std::str::FromStr;

pub enum OckamdWorker {
    StdinWorker(StdinWorker),
    Worker(Worker),
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
            wtx.send(OckamCommand::Worker(WorkerCommand::AddLine(buf.clone())));
        }
        buf.clear();
    }
}

impl<'a> Node<'a> {
    pub fn new(config: &'a Config) -> (Self, Sender<OckamCommand>) {
        // TODO: temporarily passed into the node, need to re-work
        let (router_tx, router_rx) = std::sync::mpsc::channel();
        let router = Router::new(router_rx);

        // create the vault, using the FILESYSTEM implementation
        let mut vault =
            FilesystemVault::new(config.vault_path()).expect("failed to initialize vault");

        let mut resp_key_ctx = None;
        // check for re-use of provided identity name from CLI args, if not in on-disk in vault
        // generate a new one to be used
        if !contains_key(&mut vault, &config.identity_name()) {
            // if responder, generate keypair and display static public key
            if matches!(config.role(), Role::Sink) {
                let attributes = SecretKeyAttributes {
                    xtype: SecretKeyType::Curve25519,
                    purpose: SecretPurposeType::KeyAgreement,
                    persistence: SecretPersistenceType::Persistent,
                };
                resp_key_ctx = Some(
                    vault
                        .secret_generate(attributes)
                        .expect("failed to generate secret"),
                );
            }
        } else {
            resp_key_ctx =
                Some(as_key_ctx(&config.identity_name()).expect("invalid identity name provided"));
        }

        if matches!(config.role(), Role::Sink) && resp_key_ctx.is_some() {
            if let Ok(resp_key) = vault.secret_public_key_get(resp_key_ctx.unwrap()) {
                println!("Responder public key: {}", hex::encode(resp_key));
            }
        }

        if matches!(config.role(), Role::Router) && resp_key_ctx.is_some() {
            if let Ok(resp_key) = vault.secret_public_key_get(resp_key_ctx.unwrap()) {
                println!("Router public key: {}", hex::encode(resp_key));
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

        // create the transport, currently TCP-only
        // if role == Router, give it a listen address
        let mut listen_addr: Option<SocketAddr> = None;
        if matches!(config.role(), Role::Router) {
            let la = config
                .route_hub()
                .expect("router role requires route-hub address");
            listen_addr = Some(la);
        }
        let transport_router_tx = router_tx.clone();
        let (transport_tx, transport_rx) = mpsc::channel();
        let self_transport_tx = transport_tx.clone();
        let mut transport = TcpManager::new(
            transport_rx,
            transport_tx,
            transport_router_tx,
            listen_addr,
            None,
        )
        .expect("failed to create udp transport");

        // connect to router or sink
        if matches!(config.role(), Role::Source)
            || (matches!(config.role(), Role::Sink) && config.route_hub().is_some())
        {
            let mut hop1: RouterAddress;
            if matches!(config.role(), Role::Source) {
                hop1 = config.onward_route().unwrap().addresses[0].clone();
            } else {
                let a = Address::TcpAddress(config.route_hub().unwrap());
                hop1 = RouterAddress::from_address(a).unwrap();
            }
            let sock_addr = SocketAddr::from_str(&hop1.address.as_string()).unwrap();
            let mut hop1 = match transport.connect(sock_addr) {
                Ok(h) => h,
                Err(_) => {
                    panic!("failed to connect, is server running?");
                }
            };
        }

        let node_router_tx = router_tx.clone();

        // create the worker
        let mut worker: Option<OckamdWorker> = None;
        if matches!(config.role(), Role::Source) {
            worker = Some(OckamdWorker::StdinWorker(
                StdinWorker::initialize(config, router_tx.clone(), channel_tx.clone()).unwrap(),
            ));
        }
        if matches!(config.role(), Role::Sink) {
            let worker_addr = RouterAddress::worker_router_address_from_str("01242020").unwrap();
            worker = Some(OckamdWorker::Worker(
                Worker::initialize(&config, worker_addr, router_tx.clone(), channel_tx.clone())
                    .unwrap(),
            ));
        }
        (
            Self {
                config,
                worker,
                router,
                router_tx,
                chan_manager,
                transport_tx: self_transport_tx,
                transport,
                channel_tx,
            },
            node_router_tx,
        )
    }

    pub fn run(mut self) {
        match self.worker {
            Some(mut worker) => match worker {
                OckamdWorker::Worker(mut w) => {
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

fn as_key_ctx(key_name: &str) -> Result<SecretKeyContext, String> {
    if let Some(id) = key_name.strip_suffix(cli::FILENAME_KEY_SUFFIX) {
        return Ok(SecretKeyContext::Memory(
            id.parse().map_err(|_| format!("bad key name"))?,
        ));
    }

    Err("invalid key name format".into())
}

fn contains_key(v: &mut dyn DynVault, key_name: &str) -> bool {
    if let Ok(ctx) = as_key_ctx(key_name) {
        return v.secret_export(ctx).is_ok();
    }

    false
}
