use crate::secure_channel::{Addresses, RemoteRoute};
use crate::{Identifier, SecureChannelOptions};
use core::fmt;
use core::fmt::Formatter;
use minicbor::{CborLen, Decode, Encode};
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::flow_control::{FlowControlId, FlowControls};
use ockam_core::{route, Address, Result, Route};
use serde::Serialize;

/// Result of [`super::SecureChannels::create_secure_channel()`] call.
#[derive(Debug, Clone)]
pub struct SecureChannel {
    flow_controls: FlowControls,
    their_identifier: Identifier,
    encryptor_remote_route: Arc<RwLock<RemoteRoute>>,
    addresses: Addresses,
    is_key_exchange_only: bool,
    flow_control_id: FlowControlId,
}

impl From<SecureChannel> for Address {
    fn from(value: SecureChannel) -> Self {
        value.addresses.encryptor
    }
}

impl fmt::Display for SecureChannel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Encryptor: {}, FlowId: {}",
            self.addresses.encryptor, self.flow_control_id
        )
    }
}

impl SecureChannel {
    /// Constructor.
    pub(crate) fn new(
        flow_controls: FlowControls,
        their_identifier: Identifier,
        encryptor_remote_route: Arc<RwLock<RemoteRoute>>,
        addresses: Addresses,
        is_key_exchange_only: bool,
        flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            flow_controls,
            their_identifier,
            encryptor_remote_route,
            addresses,
            is_key_exchange_only,
            flow_control_id,
        }
    }
    /// [`Address`] of the corresponding`EncryptorWorker` Worker that can be used in a route
    /// to encrypt and send a message to the other party
    pub fn encryptor_address(&self) -> &Address {
        &self.addresses.encryptor
    }
    /// Freshly generated [`FlowControlId`]
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }
    /// API [`Address`] of the corresponding`EncryptorWorker` Worker that can be used to encrypt
    /// a message without sending it
    pub fn encryptor_api_address(&self) -> &Address {
        &self.addresses.encryptor_api
    }
    /// API [`Address`] of the corresponding `DecryptorWorker` Worker that can be used to decrypt
    /// a message without sending it
    pub fn decryptor_api_address(&self) -> &Address {
        &self.addresses.decryptor_api
    }
    /// Remote messaging [`Address`] of the corresponding `DecryptorWorker`
    pub fn decryptor_remote_address(&self) -> &Address {
        &self.addresses.decryptor_remote
    }
    /// Update route to the node on the other side in case transport changes happened
    pub fn update_remote_node_route(&self, new_route: Route) -> Result<()> {
        // TODO: Maybe we also need to send a dummy message so that the other side is immediately
        //  notified about the new route (maybe we even need to ack that). But for now it's fine
        //  as it is

        let next = new_route.next().ok().cloned();

        let mut remote_route = self.encryptor_remote_route.write().unwrap();

        let old_route = remote_route.clone();

        let their_decryptor_address = old_route.route.recipient()?;
        let new_route = route![new_route, their_decryptor_address];

        remote_route.route = new_route;

        if let Some(next) = next {
            // TODO: might be useful to disable the old route eventually?
            //  Not clear if it's mandatory, but certainly will cause problems with messages that will
            //  arrive late through the old route
            SecureChannelOptions::setup_flow_control_consumer(
                &self.flow_controls,
                &self.addresses,
                &next,
            );
        }

        Ok(())
    }
    /// This secure channel is used only for handshake, further encryption happens using
    /// api address. Encryption part may be absent.
    pub fn is_key_exchange_only(&self) -> bool {
        self.is_key_exchange_only
    }
    /// The Identifier of the other side
    pub fn their_identifier(&self) -> &Identifier {
        &self.their_identifier
    }
}

/// Result of [`super::SecureChannels::create_secure_channel_listener()`] call.
#[derive(Debug, Clone, Encode, Decode, CborLen, Serialize)]
#[rustfmt::skip]
#[cbor(map)]
pub struct SecureChannelListener {
    #[n(1)] address: Address,
    #[n(2)] flow_control_id: FlowControlId,
    #[n(3)] is_key_exchange_only: bool,
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
    pub fn new(
        address: Address,
        is_key_exchange_only: bool,
        flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            address,
            is_key_exchange_only,
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
    /// This secure channel listener is used only for handshake, further encryption happens using
    /// api address. Encryption part may be absent.
    pub fn is_key_exchange_only(&self) -> bool {
        self.is_key_exchange_only
    }
}
