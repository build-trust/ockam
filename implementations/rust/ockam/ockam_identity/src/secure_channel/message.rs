use minicbor::{Decode, Encode};
use ockam_core::Route;

/// Secure Channel Message format.
#[derive(Debug, Encode, Decode, Clone)]
#[rustfmt::skip]
pub enum SecureChannelMessage {
    /// Encrypted payload message.
    #[n(0)] Payload(#[n(0)] PlaintextPayloadMessage),
    /// Close the channel.
    #[n(1)] Close,
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
    #[n(2)] pub payload: Vec<u8>,
}
