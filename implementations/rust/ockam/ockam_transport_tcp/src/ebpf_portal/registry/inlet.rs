use crate::ebpf_portal::{ConnectionIdentifier, ParsedRawSocketPacket, Port};
use crate::portal::InletSharedState;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::sync::RwLock as SyncRwLock;
use ockam_core::{Address, LocalInfoIdentifier};
use ockam_node::compat::asynchronous::RwLock as AsyncRwLock;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use tokio::net::TcpListener;
use tokio::sync::mpsc::Sender;

/// Inlet registry
#[derive(Default, Clone)]
pub(crate) struct InletRegistry {
    inlets: Arc<SyncRwLock<HashMap<Port, Inlet>>>,
}

impl InletRegistry {
    /// Get inlets
    pub fn get_inlet(&self, dst_port: Port) -> Option<Inlet> {
        let inlets = self.inlets.read().unwrap();

        inlets.get(&dst_port).cloned()
    }

    /// Create inlet
    pub fn create_inlet(
        &self,
        remote_worker_address: Address,
        internal_processor_address: Address,
        sender: Sender<ParsedRawSocketPacket>,
        port: Port,
        tcp_listener: TcpListener,
        inlet_shared_state: Arc<AsyncRwLock<InletSharedState>>,
    ) -> Inlet {
        let mut inlets = self.inlets.write().unwrap();

        let inlet_info = Inlet {
            remote_worker_address,
            internal_processor_address,
            sender,
            port,
            inlet_shared_state,
            _tcp_listener: Arc::new(tcp_listener),
            connections1: Default::default(),
            connections2: Default::default(),
        };

        inlets.insert(port, inlet_info.clone());

        inlet_info
    }

    /// Delete the inlet
    pub fn delete_inlet(&self, port: Port) {
        let mut inlets = self.inlets.write().unwrap();

        inlets.remove(&port);
    }
}

/// Inlet info
#[derive(Clone)]
pub struct Inlet {
    /// RemoteWorker Address
    pub remote_worker_address: Address,
    /// InternalProcessor Address
    pub internal_processor_address: Address,
    /// Sender to the InternalProcessor
    pub sender: Sender<ParsedRawSocketPacket>,
    /// Port
    pub port: Port,
    /// Route to the corresponding Outlet
    pub inlet_shared_state: Arc<AsyncRwLock<InletSharedState>>,
    /// Hold to mark the port as taken
    pub _tcp_listener: Arc<TcpListener>,
    /// Same map with different key
    connections1: Arc<SyncRwLock<HashMap<InletConnectionKey1, Arc<InletConnection>>>>,
    connections2: Arc<SyncRwLock<HashMap<InletConnectionKey2, Arc<InletConnection>>>>,
}

impl Inlet {
    /// Add new mapping
    pub fn add_connection(&self, connection: Arc<InletConnection>) {
        self.connections1.write().unwrap().insert(
            InletConnectionKey1 {
                client_ip: connection.client_ip,
                client_port: connection.client_port,
            },
            connection.clone(),
        );
        self.connections2.write().unwrap().insert(
            InletConnectionKey2 {
                their_identifier: connection.their_identifier.clone(),
                connection_identifier: connection.connection_identifier.clone(),
            },
            connection,
        );
    }

    /// Get mapping
    pub fn get_connection_internal(
        &self,
        client_ip: Ipv4Addr,
        client_port: Port,
    ) -> Option<Arc<InletConnection>> {
        self.connections1
            .read()
            .unwrap()
            .get(&InletConnectionKey1 {
                client_ip,
                client_port,
            })
            .cloned()
    }

    /// Get mapping
    pub(crate) fn get_connection_external(
        &self,
        their_identifier: Option<LocalInfoIdentifier>, // Identity
        connection_identifier: ConnectionIdentifier,
    ) -> Option<Arc<InletConnection>> {
        self.connections2
            .read()
            .unwrap()
            .get(&InletConnectionKey2 {
                their_identifier,
                connection_identifier,
            })
            .cloned()
    }
}

#[derive(Hash, PartialEq, Eq)]
struct InletConnectionKey1 {
    client_ip: Ipv4Addr,
    client_port: Port,
}

#[derive(Hash, PartialEq, Eq)]
struct InletConnectionKey2 {
    their_identifier: Option<LocalInfoIdentifier>,
    connection_identifier: ConnectionIdentifier,
}

/// Inlet Mapping
pub struct InletConnection {
    /// Identity Identifier of the other side
    pub their_identifier: Option<LocalInfoIdentifier>,
    /// Unique connection Identifier
    pub connection_identifier: ConnectionIdentifier,
    /// We can listen of multiple IPs
    pub inlet_ip: Ipv4Addr,
    /// Client IP
    pub client_ip: Ipv4Addr,
    /// Client port
    pub client_port: Port,
}
