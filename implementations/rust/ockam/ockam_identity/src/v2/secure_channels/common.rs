use core::fmt;
use core::fmt::Formatter;
use ockam_core::flow_control::FlowControlId;
use ockam_core::Address;

/// Result of [`super::SecureChannels::create_secure_channel()`] call.
#[derive(Debug, Clone)]
pub struct SecureChannel {
    encryptor_address: Address,
    encryptor_api_address: Address,
    flow_control_id: FlowControlId,
}

impl From<SecureChannel> for Address {
    fn from(value: SecureChannel) -> Self {
        value.encryptor_address
    }
}

impl fmt::Display for SecureChannel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Encryptor: {}, FlowId: {}",
            self.encryptor_address, self.flow_control_id
        )
    }
}

impl SecureChannel {
    /// Constructor.
    pub fn new(
        encryptor_address: Address,
        encryptor_api_address: Address,
        flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            encryptor_address,
            encryptor_api_address,
            flow_control_id,
        }
    }
    /// [`Address`] of the corresponding`EncryptorWorker` Worker that can be used in a route
    /// to encrypt and send a message to the other party
    pub fn encryptor_address(&self) -> &Address {
        &self.encryptor_address
    }
    /// Freshly generated [`FlowControlId`]
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }
    /// API [`Address`] of the corresponding`EncryptorWorker` Worker that can be used to encrypt
    /// a message without sending it
    pub fn encryptor_api_address(&self) -> &Address {
        &self.encryptor_api_address
    }
}

/// Result of [`super::SecureChannels::create_secure_channel_listener()`] call.
#[derive(Debug, Clone)]
pub struct SecureChannelListener {
    address: Address,
    flow_control_id: FlowControlId,
}

impl fmt::Display for SecureChannelListener {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Worker: {}, FlowId: {}",
            self.address, self.flow_control_id
        )
    }
}

impl SecureChannelListener {
    /// Constructor.
    pub fn new(address: Address, flow_control_id: FlowControlId) -> Self {
        Self {
            address,
            flow_control_id,
        }
    }
    /// [`Address`] of the corresponding
    /// [`SecureChannelListener`](super::super::SecureChannelListener) Worker that can be used
    /// to stop it
    pub fn address(&self) -> &Address {
        &self.address
    }
    /// Freshly generated [`FlowControlId`]
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }
}
