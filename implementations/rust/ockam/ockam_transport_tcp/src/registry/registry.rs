use crate::registry::internal::InternalRegistry;
use crate::{TcpListenerInfo, TcpReceiverInfo, TcpSenderInfo};
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::{Arc, RwLock};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;

/// Registry of all active workers and processors in TCP Transport to ease their lifecycle management
#[derive(Default, Clone, Debug)]
pub struct TcpRegistry {
    pub(super) registry: Arc<RwLock<InternalRegistry>>,
    pub(super) sni_registry: Arc<RwLock<HashMap<String, Sender<(TcpStream, SocketAddr)>>>>,
}

impl TcpRegistry {
    /// Return [`Address`]es of all active sender workers
    pub fn get_all_sender_workers(&self) -> Vec<TcpSenderInfo> {
        self.registry.read().unwrap().sender_workers.clone()
    }

    /// Return [`Address`]es of all active receiver processors
    pub fn get_all_receiver_processors(&self) -> Vec<TcpReceiverInfo> {
        self.registry.read().unwrap().receiver_processors.clone()
    }

    /// Return [`Address`]es of all active sender workers
    pub fn get_all_listeners(&self) -> Vec<TcpListenerInfo> {
        self.registry.read().unwrap().listener_processors.clone()
    }

    pub fn get_sni_listener(&self, sni: &String) -> Option<Sender<(TcpStream, SocketAddr)>> {
        self.sni_registry.read().unwrap().get(sni).cloned()
    }

    pub fn add_sni_listener(&self, sni: String, sender: Sender<(TcpStream, SocketAddr)>) {
        self.sni_registry.write().unwrap().insert(sni, sender);
    }

    pub fn remove_sni_listener(&self, sni: &String) {
        self.sni_registry.write().unwrap().remove(sni);
    }
}
