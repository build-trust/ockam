use crate::identity::{IdentityError, IdentityIdentifier};
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::compat::vec::Vec;
use ockam_core::flow_control::FlowControlId;
use ockam_core::{Address, Result, Route};

/// Known information about particular SecureChannelListener
#[derive(Clone, Debug)]
pub struct SecureChannelListenerRegistryEntry {
    address: Address,
    my_id: IdentityIdentifier,
    flow_control_id: FlowControlId,
}

impl SecureChannelListenerRegistryEntry {
    /// Create new registry entry
    pub fn new(
        address: Address,
        my_id: IdentityIdentifier,
        flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            address,
            my_id,
            flow_control_id,
        }
    }

    /// Listener Address
    pub fn address(&self) -> &Address {
        &self.address
    }

    /// Listener [`IdentityIdentifier`]
    pub fn my_id(&self) -> IdentityIdentifier {
        self.my_id.clone()
    }

    /// [`FlowControlId`]
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }
}

/// Known information about particular SecureChannel
#[derive(Clone, Debug)]
pub struct SecureChannelRegistryEntry {
    encryptor_messaging_address: Address,
    encryptor_api_address: Address,
    decryptor_messaging_address: Address,
    decryptor_api_address: Address,
    is_initiator: bool,
    my_id: IdentityIdentifier,
    their_id: IdentityIdentifier,
    their_decryptor_address: Address,
    route: Route,
    authorized_identifiers: Option<Vec<IdentityIdentifier>>,
    flow_control_id: FlowControlId,
}

impl SecureChannelRegistryEntry {
    /// Create new registry entry
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        encryptor_messaging_address: Address,
        encryptor_api_address: Address,
        decryptor_messaging_address: Address,
        decryptor_api_address: Address,
        is_initiator: bool,
        my_id: IdentityIdentifier,
        their_id: IdentityIdentifier,
        their_decryptor_address: Address,
        route: Route,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
        flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            encryptor_messaging_address,
            encryptor_api_address,
            decryptor_messaging_address,
            decryptor_api_address,
            is_initiator,
            my_id,
            their_id,
            their_decryptor_address,
            route,
            authorized_identifiers,
            flow_control_id,
        }
    }

    /// Encryptor messaging address
    pub fn encryptor_messaging_address(&self) -> &Address {
        &self.encryptor_messaging_address
    }

    /// Encryptor api address
    pub fn encryptor_api_address(&self) -> &Address {
        &self.encryptor_api_address
    }

    /// Decryptor messaging address
    pub fn decryptor_messaging_address(&self) -> &Address {
        &self.decryptor_messaging_address
    }

    /// Decryptor api address
    pub fn decryptor_api_address(&self) -> &Address {
        &self.decryptor_api_address
    }

    /// If we are were initiating this channel
    pub fn is_initiator(&self) -> bool {
        self.clone().is_initiator
    }

    /// Our `IdentityIdentifier`
    pub fn my_id(&self) -> IdentityIdentifier {
        self.my_id.clone()
    }

    /// Their `IdentityIdentifier`
    pub fn their_id(&self) -> IdentityIdentifier {
        self.their_id.clone()
    }

    /// Their `Decryptor` address
    pub fn their_decryptor_address(&self) -> Address {
        self.their_decryptor_address.clone()
    }

    /// Route to the remote
    pub fn route(&self) -> &Route {
        &self.route
    }

    /// Set of authorized identifiers. FIXE
    pub fn authorized_identifiers(&self) -> &Option<Vec<IdentityIdentifier>> {
        &self.authorized_identifiers
    }

    /// [`FlowControlId`]
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }
}

#[derive(Clone, Default)]
struct SecureChannelRegistryInternal {
    // Encryptor address is used as a key
    channels: BTreeMap<Address, SecureChannelRegistryEntry>,
    listeners: BTreeMap<Address, SecureChannelListenerRegistryEntry>,
}

/// Registry of all known Secure Channels
#[derive(Clone, Default)]
pub struct SecureChannelRegistry {
    // Encryptor address is used as a key
    registry: Arc<RwLock<SecureChannelRegistryInternal>>,
}

impl SecureChannelRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            registry: Default::default(),
        }
    }
}

impl SecureChannelRegistry {
    /// Register new SecureChannel in that registry
    pub fn register_channel(&self, info: SecureChannelRegistryEntry) -> Result<()> {
        let res = self
            .registry
            .write()
            .unwrap()
            .channels
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
        self.registry
            .write()
            .unwrap()
            .channels
            .remove(encryptor_address)
    }

    /// Register new SecureChannelListener in that registry
    pub fn register_listener(&self, info: SecureChannelListenerRegistryEntry) -> Result<()> {
        let res = self
            .registry
            .write()
            .unwrap()
            .listeners
            .insert(info.address.clone(), info);

        if res.is_some() {
            return Err(IdentityError::DuplicateSecureChannelListener.into());
        }

        Ok(())
    }

    /// Unregister a SecureChannelListener and return removed `SecureChannelListenerRegistryEntry`
    pub fn unregister_listener(
        &self,
        address: &Address,
    ) -> Option<SecureChannelListenerRegistryEntry> {
        self.registry.write().unwrap().listeners.remove(address)
    }

    /// Get list of all known SecureChannels
    pub fn get_channel_list(&self) -> Vec<SecureChannelRegistryEntry> {
        self.registry
            .read()
            .unwrap()
            .channels
            .values()
            .cloned()
            .collect()
    }

    /// Get SecureChannel with given encryptor messaging address
    pub fn get_channel_by_encryptor_address(
        &self,
        encryptor_address: &Address,
    ) -> Option<SecureChannelRegistryEntry> {
        self.registry
            .read()
            .unwrap()
            .channels
            .get(encryptor_address)
            .cloned()
    }

    /// Get SecureChannel with given decryptor messaging address
    pub fn get_channel_by_decryptor_address(
        &self,
        decryptor_address: &Address,
    ) -> Option<SecureChannelRegistryEntry> {
        self.registry
            .read()
            .unwrap()
            .channels
            .iter()
            .find(|(_, entry)| entry.decryptor_messaging_address == *decryptor_address)
            .map(|(_, entry)| entry.clone())
    }

    /// Get list of all known SecureChannelListeners
    pub fn get_listener_list(&self) -> Vec<SecureChannelListenerRegistryEntry> {
        self.registry
            .read()
            .unwrap()
            .listeners
            .values()
            .cloned()
            .collect()
    }

    /// Get SecureChannelListener with given address
    pub fn get_listener_by_address(
        &self,
        address: &Address,
    ) -> Option<SecureChannelListenerRegistryEntry> {
        self.registry
            .read()
            .unwrap()
            .listeners
            .get(address)
            .cloned()
    }
}
