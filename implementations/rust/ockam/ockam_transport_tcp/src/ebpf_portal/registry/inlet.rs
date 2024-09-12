use crate::ebpf_portal::RawSocketMessage;
use crate::portal::addresses::Addresses;
use crate::portal::InletSharedState;
use crate::TcpInletOptions;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;
use tokio::sync::mpsc::Sender;

/// Inlet registry
#[derive(Default, Clone)]
pub(crate) struct InletRegistry {
    inlets: Arc<RwLock<HashMap<u16, InletInfo>>>,
    mapping: Arc<RwLock<Vec<InletMappingValue>>>,
}

impl InletRegistry {
    /// Add new mapping
    pub fn add_mapping(&self, mapping: InletMappingValue) {
        // FIXME: eBPF duplicates
        self.mapping.write().unwrap().push(mapping)
    }

    /// Get mapping
    pub fn get_mapping(&self, client_ip: Ipv4Addr, client_port: u16) -> Option<InletMappingValue> {
        let mapping = self.mapping.read().unwrap();

        mapping.iter().find_map(|x| {
            if x.client_ip == client_ip && x.client_port == client_port {
                Some(x.clone())
            } else {
                None
            }
        })
    }

    /// Get inlets
    pub fn get_inlets_info(
        &self,
        dst_port: u16,
    ) -> Option<(Arc<RwLock<InletSharedState>>, TcpInletOptions)> {
        let inlets = self.inlets.read().unwrap();

        let inlet = inlets.get(&dst_port)?;

        Some((inlet.inlet_shared_state.clone(), inlet.options.clone()))
    }

    /// Create inlet
    pub fn create_inlet(
        &self,
        options: TcpInletOptions,
        port: u16,
        tcp_listener: TcpListener,
        inlet_shared_state: Arc<RwLock<InletSharedState>>,
    ) {
        let mut inlets = self.inlets.write().unwrap();

        inlets.insert(
            port,
            InletInfo {
                options,
                inlet_shared_state,
                tcp_listener,
            },
        );
    }

    /// Delete the inlet
    pub fn delete_inlet(&self, port: u16) {
        let mut inlets = self.inlets.write().unwrap();

        inlets.remove(&port);
    }
}

/// Inlet info
pub struct InletInfo {
    /// Route to the corresponding Outlet
    pub inlet_shared_state: Arc<RwLock<InletSharedState>>,
    /// Options
    pub options: TcpInletOptions,
    /// Hold to mark the port as taken
    pub tcp_listener: TcpListener,
}

/// Inlet Mapping
#[derive(Clone)]
pub(crate) struct InletMappingValue {
    /// Client IP
    pub client_ip: Ipv4Addr,
    /// Client port
    pub client_port: u16,
    /// Addresses
    pub _addresses: Addresses,
    /// Sender to a processor
    pub sender: Sender<RawSocketMessage>,
}
