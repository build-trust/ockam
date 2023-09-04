use ockam_api::nodes::models::portal::OutletStatus;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// The ModelState stores all the data which is not maintained by the NodeManager.
#[derive(Serialize, Deserialize, Clone)]
pub struct ModelState {
    #[serde(default = "Vec::new")]
    pub(crate) tcp_outlets: Vec<OutletStatus>,
}

impl Default for ModelState {
    fn default() -> Self {
        ModelState::new(vec![])
    }
}

impl ModelState {
    pub fn new(tcp_outlets: Vec<OutletStatus>) -> Self {
        Self { tcp_outlets }
    }

    pub fn get_outlet_status_by_socket_addr(
        &self,
        socket_addr: &SocketAddr,
    ) -> Option<OutletStatus> {
        self.tcp_outlets
            .iter()
            .find(|o| o.socket_addr == *socket_addr)
            .cloned()
    }
}
