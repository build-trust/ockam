use crate::nodes::models::portal::{InletStatus, OutletStatus};
use ockam_core::Result;
use ockam_core::{async_trait, Address};

/// The TcpPortalsRepository is responsible for accessing the configured tcp inlets and tcp outlets
#[async_trait]
pub trait TcpPortalsRepository: Send + Sync + 'static {
    async fn store_tcp_inlet(&self, node_name: &str, tcp_inlet_status: &InletStatus) -> Result<()>;
    async fn get_tcp_inlet(&self, node_name: &str, alias: &str) -> Result<Option<InletStatus>>;
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
