use crate::incoming_services::PersistentIncomingServiceState;
use ockam_api::nodes::models::portal::OutletStatus;
use serde::{Deserialize, Serialize};

/// The ModelState stores all the data which is not maintained by the NodeManager.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ModelState {
    #[serde(default = "Vec::new")]
    pub(crate) tcp_outlets: Vec<OutletStatus>,

    #[serde(default = "Vec::new")]
    pub(crate) incoming_services: Vec<PersistentIncomingServiceState>,
}

impl Default for ModelState {
    fn default() -> Self {
        ModelState::new(vec![], vec![])
    }
}

impl ModelState {
    pub fn new(
        tcp_outlets: Vec<OutletStatus>,
        incoming_services: Vec<PersistentIncomingServiceState>,
    ) -> Self {
        Self {
            tcp_outlets,
            incoming_services,
        }
    }
}
