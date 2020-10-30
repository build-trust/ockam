use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

use crate::cli;
use crate::config::{Config, Role};
use crate::worker::Worker;

use ockam_channel::*;
use ockam_kex::{
    xx::{XXInitiator, XXNewKeyExchanger, XXResponder},
    CipherSuite,
};
use ockam_message::message::AddressType;
use ockam_router::router::Router;
use ockam_system::commands::{OckamCommand, RouterCommand};
use ockam_transport::transport::UdpTransport;
use ockam_vault::types::*;
use ockam_vault::{file::FilesystemVault, DynVault};

#[allow(dead_code)]
pub struct Node<'a> {
    config: &'a Config,
    chan_manager: ChannelManager<XXInitiator, XXResponder, XXNewKeyExchanger>,
    worker: Option<Worker>,
    router: Router,
    router_tx: Sender<OckamCommand>,
    transport: UdpTransport,
    transport_tx: Sender<OckamCommand>,
    pub channel_tx: Sender<OckamCommand>,
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
            if matches!(config.role(), Role::Responder) {
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

        if matches!(config.role(), Role::Responder) && resp_key_ctx.is_some() {
            if let Ok(resp_key) = vault.secret_public_key_get(resp_key_ctx.unwrap()) {
                println!("Responder public key: {}", hex::encode(resp_key));
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

        // create the transport, currently UDP-only
        let transport_router_tx = router_tx.clone();
        let (transport_tx, transport_rx) = mpsc::channel();
        let self_transport_tx = transport_tx.clone();
        let transport = UdpTransport::new(
            transport_rx,
            transport_tx,
            transport_router_tx,
            config.local_socket().to_string().as_str(),
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
                channel_tx,
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

        println!("worker registered");
        self.worker = Some(worker);
    }

    pub fn run(mut self) {
        match self.worker {
            Some(mut worker) => {
                println!("will poll worker");
                while self.router.poll()
                    && self.transport.poll()
                    && worker.poll()
                    && self
                        .chan_manager
                        .poll()
                        .expect("channel manager poll failure")
                {
                    thread::sleep(time::Duration::from_millis(1));
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
