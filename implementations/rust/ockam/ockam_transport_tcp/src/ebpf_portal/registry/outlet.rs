use crate::ebpf_portal::RawSocketMessage;
use crate::portal::addresses::Addresses;
use ockam_core::Address;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::Sender;

/// Outlet registry
#[derive(Default, Clone)]
pub struct OutletRegistry {
    targets_ports: Arc<RwLock<HashMap<OutletKey, OutletInfo>>>,
    mapping: Arc<RwLock<Vec<OutletMappingValue>>>,
}

impl OutletRegistry {
    /// Add mapping
    pub(crate) fn add_mapping(&self, mapping: OutletMappingValue) {
        // FIXME: eBPF duplicates
        self.mapping.write().unwrap().push(mapping)
    }

    /// Get mapping
    pub(crate) fn get_mapping(&self, dst_port: u16) -> Option<OutletMappingValue> {
        let mapping = self.mapping.read().unwrap();

        mapping.iter().find_map(|x| {
            if x.assigned_port == dst_port {
                Some(x.clone())
            } else {
                None
            }
        })
    }

    /// Get mapping
    pub(crate) fn get_mapping2(
        &self,
        inlet_worker_address: &Address,
    ) -> Option<OutletMappingValue> {
        let mapping = self.mapping.read().unwrap();

        mapping.iter().find_map(|x| {
            if &x.inlet_worker_address == inlet_worker_address {
                Some(x.clone())
            } else {
                None
            }
        })
    }

    /// Get outlet
    pub fn get_outlet(&self, src_ip: Ipv4Addr, src_port: u16) -> Option<OutletInfo> {
        self.targets_ports
            .read()
            .unwrap()
            .get(&OutletKey {
                dst_ip: src_ip,
                dst_port: src_port,
            })
            .cloned()
    }

    /// Add outlet
    pub fn add_outlet(&self, dst_ip: Ipv4Addr, dst_port: u16) {
        // TODO: eBPF Duplicates?
        self.targets_ports
            .write()
            .unwrap()
            .insert(OutletKey { dst_ip, dst_port }, OutletInfo {});
    }
}

#[derive(Hash, PartialEq, Eq)]
struct OutletKey {
    dst_ip: Ipv4Addr,
    dst_port: u16,
}

/// Outlet info
#[derive(Clone)]
pub struct OutletInfo {}

/// Outlet mapping
#[derive(Clone)]
pub(crate) struct OutletMappingValue {
    // TODO: eBPF Add identifier?
    /// The other side's Inlet worker address
    pub inlet_worker_address: Address,
    /// Assigned port on our machine for a specific connection
    pub assigned_port: u16,
    /// Addresses
    pub _addresses: Addresses,
    /// Sender to the processor
    pub sender: Sender<RawSocketMessage>,
}
