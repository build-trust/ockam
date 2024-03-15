use crate::nodes::models::portal::{InletStatus, OutletStatus};
use ockam_core::async_trait;
use ockam_core::Result;

/// The TcpPortalsRepository is responsible for accessing the configured tcp inlets and tcp outlets
#[async_trait]
pub trait TcpPortalsRepository: Send + Sync + 'static {
    async fn store_tcp_inlet(&self, node_name: &str, tcp_inlet_status: &InletStatus) -> Result<()>;
    async fn store_tcp_outlet(&self, node_name: &str, alias: &str, tcp_outlet_status: &OutletStatus) -> Result<()>;

    async fn get_tcp_inlet(&self, node_name: &str, alias: &str) -> Result<Option<InletStatus>>;
    async fn get_tcp_outlet(&self, node_name: &str, alias: &str) -> Result<Option<OutletStatus>>;

    async fn delete_tcp_inlet(&self, node_name: &str, alias: &str) -> Result<()>;
    async fn delete_tcp_outlet(&self, node_name: &str, alias: &str) -> Result<()>;
}
