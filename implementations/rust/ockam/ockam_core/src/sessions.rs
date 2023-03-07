//! Sessions
//!
//! Allows to setup Message Flow Authorization between two workers (in this case
//! SecureChannel Decryptor and Tcp Receiver) to limit their interaction with other workers
//! for security reasons.

use crate::compat::rand::random;
use crate::compat::sync::{Arc, RwLock};
use crate::compat::vec::Vec;
use crate::Address;

mod access_control;
mod local_info;
mod session_id;

pub use access_control::*;
pub use local_info::*;
pub use session_id::*;

// TODO: Consider integrating this into Routing for better UX + to allow removing
// entries from that storage
/// Storage for Session-related data tied to an [`Address`]
#[derive(Clone, Debug, Default)]
pub struct Sessions {
    internal: Arc<RwLock<SessionsInternal>>,
}

impl Sessions {
    /// Generate a fresh random [`SessionId`]
    pub fn generate_session_id(&self) -> SessionId {
        random()
    }

    /// Mark that given [`Address`] belongs to the given [`SessionId`]
    pub fn set_session_id(&self, address: &Address, session_id: &SessionId) {
        let mut lock = self.internal.write().unwrap();
        let address_info = lock.get_info_mut(address);

        address_info.session_id = Some(session_id.clone());
    }

    /// Get [`SessionId`] for given [`Address`]
    pub fn get_session_id(&self, address: &Address) -> Option<SessionId> {
        match self.internal.read().unwrap().get_info(address) {
            Some(address_info) => address_info.session_id.clone(),
            None => None,
        }
    }

    /// Mark that given [`Address`] belongs to the given listener [`SessionId`]
    pub fn set_listener_session_id(&self, address: &Address, session_id: &SessionId) {
        let mut lock = self.internal.write().unwrap();
        let address_info = lock.get_info_mut(address);

        address_info.listener_session_id = Some(session_id.clone());
    }

    /// Get listener [`SessionId`] for given [`Address`]
    pub fn get_listener_session_id(&self, address: &Address) -> Option<SessionId> {
        match self.internal.read().unwrap().get_info(address) {
            Some(address_info) => address_info.listener_session_id.clone(),
            None => None,
        }
    }
}

#[derive(Clone, Debug)]
struct AddressInfo {
    address: Address,
    session_id: Option<SessionId>,
    listener_session_id: Option<SessionId>,
}

impl AddressInfo {
    fn new(
        address: Address,
        session_id: Option<SessionId>,
        listener_session_id: Option<SessionId>,
    ) -> Self {
        Self {
            address,
            session_id,
            listener_session_id,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct SessionsInternal {
    data: Vec<AddressInfo>,
}

impl SessionsInternal {
    fn get_info_mut(&mut self, address: &Address) -> &mut AddressInfo {
        if !self.data.iter().any(|x| &x.address == address) {
            let info = AddressInfo::new(address.clone(), None, None);
            self.data.push(info);
        }

        self.data
            .iter_mut()
            .find(|x| &x.address == address)
            .unwrap()
    }

    fn get_info(&self, address: &Address) -> Option<&AddressInfo> {
        self.data.iter().find(|&x| &x.address == address)
    }
}
