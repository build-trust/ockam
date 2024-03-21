use ockam_core::{Address, Route};
use ockam_multiaddr::MultiAddr;
use std::net::SocketAddr;

use crate::nodes::models::portal::{InletStatus, OutletStatus};
use crate::Result;
use crate::{CliState, ConnectionStatus};

impl CliState {
    /// Create a TCP inlet
    #[instrument(skip_all)]
    pub async fn create_tcp_inlet(
        &self,
        node_name: &str,
        bind_addr: &str,
        worker_addr: &Option<Address>,
        alias: &str,
        payload: &Option<String>,
        outlet_route: &Option<Route>,
        connection_status: ConnectionStatus,
        outlet_addr: &MultiAddr,
    ) -> Result<InletStatus> {
        let tcp_inlet_status = InletStatus::new(
            bind_addr,
            worker_addr.clone().map(|a| a.to_string()),
            alias,
            payload.clone(),
            outlet_route.clone().map(|r| r.to_string()),
            connection_status,
            outlet_addr.to_string(),
        );
        self.tcp_portals_repository()
            .store_tcp_inlet(node_name, &tcp_inlet_status)
            .await?;
        Ok(tcp_inlet_status)
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
        socket_addr: &SocketAddr,
        worker_addr: &Address,
        payload: &Option<String>,
    ) -> Result<OutletStatus> {
        let tcp_outlet_status =
            OutletStatus::new(socket_addr.clone(), worker_addr.clone(), payload.clone());

        self.tcp_portals_repository()
            .store_tcp_outlet(node_name, &tcp_outlet_status)
            .await?;
        Ok(tcp_outlet_status)
    }

    /// Delete a TCP outlet
    #[instrument(skip_all)]
    pub async fn delete_tcp_outlet(&self, node_name: &str, worker_addr: &Address) -> Result<()> {
        Ok(self
            .tcp_portals_repository()
            .delete_tcp_outlet(node_name, worker_addr)
            .await?)
    }
}
