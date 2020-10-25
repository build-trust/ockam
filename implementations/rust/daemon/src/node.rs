use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

use crate::config::{Config, Role};
use crate::vault::FilesystemVault;
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
use ockam_vault::types::{
    SecretKeyAttributes, SecretKeyType, SecretPersistenceType, SecretPurposeType,
};
use ockam_vault::DynVault;

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
        let vault = Arc::new(Mutex::new(
            FilesystemVault::new(config.vault_path()).expect("failed to initialize vault"),
        ));

        // if responder, generate keypair and display static public key
        let resp_key_opt;
        let mut resp_key_ctx_opt = None;
        // match config.role() {
        //     Role::Responder => {
        if let Role::Responder = config.role() {
            let attributes = SecretKeyAttributes {
                xtype: SecretKeyType::Curve25519,
                purpose: SecretPurposeType::KeyAgreement,
                persistence: SecretPersistenceType::Persistent,
            };
            let mut v = vault.lock().unwrap();
            let resp_key_ctx = v.secret_generate(attributes).unwrap();
            let resp_key = v.secret_public_key_get(resp_key_ctx).unwrap();
            resp_key_opt = Some(resp_key);
            resp_key_ctx_opt = Some(resp_key_ctx);
            println!("Responder public key: {}", resp_key_opt.unwrap());
        }

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
            resp_key_ctx_opt,
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
            config.local_host().to_string().as_str(),
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
