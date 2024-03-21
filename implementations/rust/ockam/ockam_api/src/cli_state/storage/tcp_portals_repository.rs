use crate::nodes::models::portal::OutletStatus;
use ockam_core::Result;
use ockam_core::{async_trait, Address};
use ockam_multiaddr::MultiAddr;
use std::net::SocketAddr;

/// The TcpPortalsRepository is responsible for accessing the configured tcp inlets and tcp outlets
#[async_trait]
pub trait TcpPortalsRepository: Send + Sync + 'static {
    async fn store_tcp_inlet(&self, node_name: &str, tcp_inlet: &TcpInlet) -> Result<()>;
    async fn get_tcp_inlet(&self, node_name: &str, alias: &str) -> Result<Option<TcpInlet>>;
    async fn delete_tcp_inlet(&self, node_name: &str, alias: &str) -> Result<()>;
    async fn store_tcp_outlet(
        &self,
        node_name: &str,
        tcp_outlet_status: &OutletStatus,
    ) -> Result<()>;

    async fn get_tcp_outlet(
        &self,
        node_name: &str,
        worker_addr: &Address,
    ) -> Result<Option<OutletStatus>>;

    async fn delete_tcp_outlet(&self, node_name: &str, worker_addr: &Address) -> Result<()>;

    async fn delete_tcp_portals_by_node(&self, node_name: &str) -> Result<()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpInlet {
    bind_addr: SocketAddr,
    outlet_addr: MultiAddr,
    alias: String,
}

impl TcpInlet {
    pub fn new(bind_addr: &SocketAddr, outlet_addr: &MultiAddr, alias: &str) -> TcpInlet {
        Self {
            bind_addr: bind_addr.clone(),
            outlet_addr: outlet_addr.clone(),
            alias: alias.to_string(),
        }
    }

    pub fn bind_addr(&self) -> SocketAddr {
        self.bind_addr.clone()
    }

    pub fn outlet_addr(&self) -> MultiAddr {
        self.outlet_addr.clone()
    }

    pub fn alias(&self) -> String {
        self.alias.clone()
    }
}
