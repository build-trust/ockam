use crate::registry::internal::InternalRegistry;
use crate::{TcpListenerInfo, TcpReceiverInfo, TcpSenderInfo};
use ockam_core::compat::sync::{Arc, RwLock};

/// Registry of all active workers and processors in TCP Transport to ease their lifecycle management
#[derive(Default, Clone, Debug)]
pub struct TcpRegistry {
    pub(super) registry: Arc<RwLock<InternalRegistry>>,
}

impl TcpRegistry {
    /// Return [`Addresses`](crate::TcpListenerInfo) of all active sender workers
    pub fn get_all_sender_workers(&self) -> Vec<TcpSenderInfo> {
        self.registry.read().unwrap().sender_workers.clone()
    }

    /// Return [`Addresses`](crate::TcpListenerInfo) of all active receiver processors
    pub fn get_all_receiver_processors(&self) -> Vec<TcpReceiverInfo> {
        self.registry.read().unwrap().receiver_processors.clone()
    }

    /// Return [`Addresses`](crate::TcpListenerInfo) of all active sender workers
    pub fn get_all_listeners(&self) -> Vec<TcpListenerInfo> {
        self.registry.read().unwrap().listener_processors.clone()
    }
}
