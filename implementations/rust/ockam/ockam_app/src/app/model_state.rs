use serde::{Deserialize, Serialize};

use ockam_api::nodes::models::portal::OutletStatus;

/// The ModelState stores all the data which is not maintained by the NodeManager:
///    - user information
///    - shared services
///    - sent invitations
///    - etc...
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
}
