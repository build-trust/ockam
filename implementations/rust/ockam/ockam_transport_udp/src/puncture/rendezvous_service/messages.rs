use minicbor::{CborLen, Decode, Encode};
use ockam_core::{Decodable, Encodable, Encoded, Message, Result};

/// Request type for UDP Puncture Rendezvous service
#[derive(Encode, Decode, CborLen, Debug)]
#[rustfmt::skip]
pub enum RendezvousRequest {
    /// Ping service to see if it is reachable and working.
    #[n(0)] Ping,
    /// Get my public IP and port
    #[n(1)] GetMyAddress,
}

impl Encodable for RendezvousRequest {
    fn encode(self) -> Result<Encoded> {
        ockam_core::cbor_encode_preallocate(self)
    }
}

impl Decodable for RendezvousRequest {
    fn decode(e: &[u8]) -> Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl Message for RendezvousRequest {}

/// Response type for UDP Puncture Rendezvous service
#[derive(Encode, Decode, CborLen, Debug)]
#[rustfmt::skip]
pub enum RendezvousResponse {
    #[n(0)] Pong,
    #[n(1)] GetMyAddress(#[n(0)] String),
}

impl Encodable for RendezvousResponse {
    fn encode(self) -> Result<Encoded> {
        ockam_core::cbor_encode_preallocate(self)
    }
}

impl Decodable for RendezvousResponse {
    fn decode(e: &[u8]) -> Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl Message for RendezvousResponse {}
