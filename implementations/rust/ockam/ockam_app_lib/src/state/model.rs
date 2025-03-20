use crate::incoming_services::PersistentIncomingService;
use ockam_api::nodes::models::portal::TcpOutletInfo;
use serde::Serialize;

/// The ModelState stores all the data which is not maintained by the NodeManager.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ModelState {
    #[serde(default = "Vec::new")]
    pub(crate) tcp_outlets: Vec<TcpOutletInfo>,

    #[serde(default = "Vec::new")]
    pub(crate) incoming_services: Vec<PersistentIncomingService>,
}

impl Default for ModelState {
    fn default() -> Self {
        ModelState::new(vec![], vec![])
    }
}

impl ModelState {
    pub fn new(
        tcp_outlets: Vec<TcpOutletInfo>,
        incoming_services: Vec<PersistentIncomingService>,
    ) -> Self {
        Self {
            tcp_outlets,
            incoming_services,
        }
    }
}
