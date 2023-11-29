use crate::models::{ChangeHistory, CredentialAndPurposeKey};
use minicbor::{Decode, Encode};
use ockam_core::compat::vec::Vec;
use ockam_core::Route;

/// Secure Channel Message format.
#[derive(Debug, Encode, Decode, Clone)]
#[rustfmt::skip]
pub enum SecureChannelMessage {
    /// Encrypted payload message.
    #[n(0)] Payload(#[n(0)] PlaintextPayloadMessage),
    /// Present credentials one more time.
    #[n(1)] RefreshCredentials(#[n(0)] RefreshCredentialsMessage),
    /// Close the channel.
    #[n(2)] Close,
}

/// Secure Channel Message format.
#[derive(Debug, Encode, Decode, Clone)]
#[rustfmt::skip]
pub struct PlaintextPayloadMessage {
    /// Onward route of the message.
    #[n(0)] pub onward_route: Route,
    /// Return route of the message.
    #[n(1)] pub return_route: Route,
    /// Untyped binary payload.
    #[cbor(with = "minicbor::bytes")]
    #[n(2)] pub payload: Vec<u8>,
}

/// Secure Channel Message format.
#[derive(Debug, Encode, Decode, Clone)]
#[rustfmt::skip]
pub struct RefreshCredentialsMessage {
    /// Exported identity
    #[n(0)] pub change_history: ChangeHistory,
    /// Credentials associated to the identity along with corresponding Credentials Purpose Keys
    /// to verify those Credentials
    #[n(1)] pub credentials: Vec<CredentialAndPurposeKey>,
}
