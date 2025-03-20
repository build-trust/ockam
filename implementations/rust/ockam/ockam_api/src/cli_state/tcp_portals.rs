use super::Result;
use crate::cli_state::TcpInlet;
use crate::nodes::models::portal::TcpOutletInfo;
use crate::CliState;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Address;
use ockam_multiaddr::MultiAddr;
use std::net::SocketAddr;

impl CliState {
    /// Create a TCP inlet
    #[instrument(skip_all)]
    pub async fn create_tcp_inlet(
        &self,
        node_name: &str,
        bind_addr: &SocketAddr,
        outlet_addr: &MultiAddr,
        alias: &str,
        privileged: bool,
    ) -> Result<TcpInlet> {
        let tcp_inlet = TcpInlet::new(bind_addr, outlet_addr, alias, privileged);
        self.tcp_portals_repository()
            .store_tcp_inlet(node_name, &tcp_inlet)
            .await?;
        Ok(tcp_inlet)
    }

    /// Get a TCP inlet by node name and alias
    #[instrument(skip_all)]
    pub async fn get_tcp_inlet(&self, node_name: &str, alias: &str) -> Result<TcpInlet> {
        Ok(self
            .tcp_portals_repository()
            .get_tcp_inlet(node_name, alias)
            .await?
            .ok_or_else(|| {
                ockam_core::Error::new(
                    Origin::Api,
                    Kind::NotFound,
                    format!("no tcp inlet found for node {node_name}, with alias {alias}"),
                )
            })?)
    }

    /// Delete a TCP inlet
    #[instrument(skip_all)]
    pub async fn delete_tcp_inlet(&self, node_name: &str, alias: &str) -> Result<()> {
        Ok(self
            .tcp_portals_repository()
            .delete_tcp_inlet(node_name, alias)
            .await?)
    }

    /// Create a TCP outlet
    #[instrument(skip_all)]
    pub async fn create_tcp_outlet(
        &self,
        node_name: &str,
        tcp_outlet_info: &TcpOutletInfo,
    ) -> Result<()> {
        self.tcp_portals_repository()
            .store_tcp_outlet(node_name, tcp_outlet_info)
            .await?;
        Ok(())
    }

    /// Delete a TCP outlet
    #[instrument(skip_all)]
    pub async fn delete_tcp_outlet(&self, node_name: &str, worker_addr: &Address) -> Result<()> {
        Ok(self
            .tcp_portals_repository()
            .delete_tcp_outlet(node_name, worker_addr)
            .await?)
    }

    //// List all TCP outlets
    #[instrument(skip_all)]
    pub async fn list_tcp_outlets(&self, node_name: &str) -> Result<Vec<TcpOutletInfo>> {
        Ok(self
            .tcp_portals_repository()
            .list_tcp_outlets(node_name)
            .await?)
    }
}
