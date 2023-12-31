use crate::incoming_services::PersistentIncomingService;
use crate::local_service::PersistentLocalService;

/// The ModelState stores all the data which is not maintained by the NodeManager.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelState {
    pub(crate) local_services: Vec<PersistentLocalService>,
    pub(crate) incoming_services: Vec<PersistentIncomingService>,
}

impl Default for ModelState {
    fn default() -> Self {
        ModelState::new(vec![], vec![])
    }
}

impl ModelState {
    pub fn new(
        local_services: Vec<PersistentLocalService>,
        incoming_services: Vec<PersistentIncomingService>,
    ) -> Self {
        Self {
            local_services,
            incoming_services,
        }
    }
}
