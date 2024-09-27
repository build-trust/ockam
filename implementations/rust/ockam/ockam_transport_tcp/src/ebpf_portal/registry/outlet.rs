use crate::ebpf_portal::{ConnectionIdentifier, Port};
use ockam_core::{Address, Route};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;

/// Outlet registry
#[derive(Default, Clone)]
pub struct OutletRegistry {
    outlets: Arc<RwLock<HashMap<OutletKey, Outlet>>>,
}

impl OutletRegistry {
    /// Get outlet
    pub fn get_outlet(&self, src_ip: Ipv4Addr, src_port: Port) -> Option<Outlet> {
        self.outlets
            .read()
            .unwrap()
            .get(&OutletKey {
                dst_ip: src_ip,
                dst_port: src_port,
            })
            .cloned()
    }

    /// Add outlet
    pub fn add_outlet(
        &self,
        portal_worker_address: Address,
        dst_ip: Ipv4Addr,
        dst_port: Port,
    ) -> Outlet {
        let outlet_info = Outlet {
            dst_ip,
            dst_port,
            portal_worker_address,
            connections1: Default::default(),
            connections2: Default::default(),
        };

        // TODO: eBPF Duplicates?
        self.outlets
            .write()
            .unwrap()
            .insert(OutletKey { dst_ip, dst_port }, outlet_info.clone());

        outlet_info
    }
}

#[derive(Hash, PartialEq, Eq)]
struct OutletKey {
    dst_ip: Ipv4Addr,
    dst_port: Port,
}

/// Outlet info
#[derive(Clone)]
pub struct Outlet {
    /// Destination IP
    pub dst_ip: Ipv4Addr,
    /// Destination Port
    pub dst_port: Port,
    /// PortalWorker Address
    pub portal_worker_address: Address,
    /// Same map with different key
    connections1: Arc<RwLock<HashMap<Port, Arc<OutletConnection>>>>,
    connections2: Arc<RwLock<HashMap<OutletConnectionKey, Arc<OutletConnection>>>>,
}

impl Outlet {
    /// Add mapping
    pub(crate) fn add_connection(&self, connection: Arc<OutletConnection>) {
        self.connections1
            .write()
            .unwrap()
            .insert(connection.assigned_port, connection.clone());
        self.connections2.write().unwrap().insert(
            OutletConnectionKey {
                identifier: connection.identifier.clone(),
                connection_identifier: connection.connection_identifier.clone(),
            },
            connection,
        );
    }

    /// Get Connection
    pub(crate) fn get_connection_internal(
        &self,
        assigned_port: Port,
    ) -> Option<Arc<OutletConnection>> {
        self.connections1
            .read()
            .unwrap()
            .get(&assigned_port)
            .cloned()
    }

    /// Get mapping
    pub(crate) fn get_connection_external(
        &self,
        identifier: Option<String>, // Identity
        connection_identifier: ConnectionIdentifier,
    ) -> Option<Arc<OutletConnection>> {
        self.connections2
            .read()
            .unwrap()
            .get(&OutletConnectionKey {
                identifier,
                connection_identifier,
            })
            .cloned()
    }
}

#[derive(Hash, PartialEq, Eq)]
struct OutletConnectionKey {
    identifier: Option<String>,
    connection_identifier: ConnectionIdentifier,
}

/// Outlet mapping
pub struct OutletConnection {
    /// Identity Identifier of the other side
    pub identifier: Option<String>,
    /// Unique connection Identifier
    pub connection_identifier: ConnectionIdentifier,
    /// Assigned port on our machine for a specific connection
    pub assigned_port: Port,
    /// Route to the other side PortalWorker
    pub return_route: Route,
    /// To hold the port
    pub _tcp_listener: Arc<TcpListener>,
}
