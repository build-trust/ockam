use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::{Address, LocalInfoIdentifier};

#[derive(Hash, Eq, PartialEq, Clone)]
pub(crate) struct MapKey {
    pub(crate) identifier: Option<LocalInfoIdentifier>,
    pub(crate) remote_address: Address,
}

#[derive(Default, Clone)]
pub struct OutletListenerRegistry {
    pub(crate) started_workers: Arc<RwLock<HashMap<MapKey, Address>>>,
}
