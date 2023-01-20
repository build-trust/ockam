use crate::error::IdentityError;
use crate::IdentityIdentifier;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::compat::vec::Vec;
use ockam_core::{Address, Result};

#[derive(Clone, Debug)]
pub struct SecureChannelRegistryEntry {
    encryptor_messaging_address: Address,
    encryptor_api_address: Address,
    decryptor_messaging_address: Address,
    decryptor_api_address: Address,
    is_initiator: bool,
    my_id: IdentityIdentifier,
    their_id: IdentityIdentifier,
}

impl SecureChannelRegistryEntry {
    pub fn new(
        encryptor_messaging_address: Address,
        encryptor_api_address: Address,
        decryptor_messaging_address: Address,
        decryptor_api_address: Address,
        is_initiator: bool,
        my_id: IdentityIdentifier,
        their_id: IdentityIdentifier,
    ) -> Self {
        Self {
            encryptor_messaging_address,
            encryptor_api_address,
            decryptor_messaging_address,
            decryptor_api_address,
            is_initiator,
            my_id,
            their_id,
        }
    }
    pub fn encryptor_messaging_address(&self) -> &Address {
        &self.encryptor_messaging_address
    }
    pub fn encryptor_api_address(&self) -> &Address {
        &self.encryptor_api_address
    }
    pub fn decryptor_messaging_address(&self) -> &Address {
        &self.decryptor_messaging_address
    }
    pub fn decryptor_api_address(&self) -> &Address {
        &self.decryptor_api_address
    }
    pub fn is_initiator(&self) -> bool {
        self.is_initiator
    }
    pub fn my_id(&self) -> &IdentityIdentifier {
        &self.my_id
    }
    pub fn their_id(&self) -> &IdentityIdentifier {
        &self.their_id
    }
}

#[derive(Clone, Default)]
pub struct SecureChannelRegistry {
    // Encryptor address is used as a key
    registry: Arc<RwLock<BTreeMap<Address, SecureChannelRegistryEntry>>>,
}

impl SecureChannelRegistry {
    pub fn new() -> Self {
        Self {
            registry: Default::default(),
        }
    }
}

impl SecureChannelRegistry {
    pub fn register_channel(&self, info: SecureChannelRegistryEntry) -> Result<()> {
        let res = self
            .registry
            .write()
            .unwrap()
            .insert(info.encryptor_messaging_address.clone(), info);

        if res.is_some() {
            return Err(IdentityError::DuplicateSecureChannel.into());
        }

        Ok(())
    }

    /// Unregister a SecureChannel and return removed `SecureChannelRegistryEntry`
    pub fn unregister_channel(
        &self,
        encryptor_address: &Address,
    ) -> Option<SecureChannelRegistryEntry> {
        self.registry.write().unwrap().remove(encryptor_address)
    pub fn get_channel_list(&self) -> Vec<SecureChannelRegistryEntry> {
        self.registry.read().unwrap().values().cloned().collect()
    }

    pub fn get_channel_by_encryptor_address(
        &self,
        encryptor_address: &Address,
    ) -> Option<SecureChannelRegistryEntry> {
        self.registry
            .read()
            .unwrap()
            .get(encryptor_address)
            .cloned()
    }
}
