use crate::nodes::models::portal::OutletStatus;
use ockam_core::Result;
use ockam_core::{async_trait, Address};
use ockam_multiaddr::MultiAddr;
use ockam_node::database::AutoRetry;
use ockam_node::retry;
use std::net::SocketAddr;

/// The TcpPortalsRepository is responsible for accessing the configured tcp inlets and tcp outlets
#[async_trait]
pub trait TcpPortalsRepository: Send + Sync + 'static {
    /// Store the configuration of a TcpInlet for a given node name
    async fn store_tcp_inlet(&self, node_name: &str, tcp_inlet: &TcpInlet) -> Result<()>;
    /// Return the configuration of a TcpInlet for a given node name and inlet alias
    async fn get_tcp_inlet(&self, node_name: &str, alias: &str) -> Result<Option<TcpInlet>>;
    /// Delete the configuration of a TcpInlet for a given node name and inlet alias
    async fn delete_tcp_inlet(&self, node_name: &str, alias: &str) -> Result<()>;

    /// Store the configuration of a TcpOutlet for a given node name
    async fn store_tcp_outlet(
        &self,
        node_name: &str,
        tcp_outlet_status: &OutletStatus,
    ) -> Result<()>;

    /// Return the configuration of a TcpOutlet for a given node name and worker address
    async fn get_tcp_outlet(
        &self,
        node_name: &str,
        worker_addr: &Address,
    ) -> Result<Option<OutletStatus>>;

    /// Delete the configuration of a TcpOutlet for a given node name and worker address
    async fn delete_tcp_outlet(&self, node_name: &str, worker_addr: &Address) -> Result<()>;
}

#[async_trait]
impl<T: TcpPortalsRepository> TcpPortalsRepository for AutoRetry<T> {
    async fn store_tcp_inlet(&self, node_name: &str, tcp_inlet: &TcpInlet) -> Result<()> {
        retry!(self.wrapped.store_tcp_inlet(node_name, tcp_inlet))
    }

    async fn get_tcp_inlet(&self, node_name: &str, alias: &str) -> Result<Option<TcpInlet>> {
        retry!(self.wrapped.get_tcp_inlet(node_name, alias))
    }

    async fn delete_tcp_inlet(&self, node_name: &str, alias: &str) -> Result<()> {
        retry!(self.wrapped.delete_tcp_inlet(node_name, alias))
    }

    async fn store_tcp_outlet(
        &self,
        node_name: &str,
        tcp_outlet_status: &OutletStatus,
    ) -> Result<()> {
        retry!(self.wrapped.store_tcp_outlet(node_name, tcp_outlet_status))
    }

    async fn get_tcp_outlet(
        &self,
        node_name: &str,
        worker_addr: &Address,
    ) -> Result<Option<OutletStatus>> {
        retry!(self.wrapped.get_tcp_outlet(node_name, worker_addr))
    }

    async fn delete_tcp_outlet(&self, node_name: &str, worker_addr: &Address) -> Result<()> {
        retry!(self.wrapped.delete_tcp_outlet(node_name, worker_addr))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpInlet {
    bind_addr: SocketAddr,
    outlet_addr: MultiAddr,
    alias: String,
    privileged: bool,
}

impl TcpInlet {
    pub fn new(
        bind_addr: &SocketAddr,
        outlet_addr: &MultiAddr,
        alias: &str,
        privileged: bool,
    ) -> TcpInlet {
        Self {
            bind_addr: *bind_addr,
            outlet_addr: outlet_addr.clone(),
            alias: alias.to_string(),
            privileged,
        }
    }

    pub fn bind_addr(&self) -> SocketAddr {
        self.bind_addr
    }

    pub fn outlet_addr(&self) -> MultiAddr {
        self.outlet_addr.clone()
    }

    pub fn alias(&self) -> String {
        self.alias.clone()
    }

    pub fn privileged(&self) -> bool {
        self.privileged
    }
}
