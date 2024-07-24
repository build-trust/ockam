use minicbor::{CborLen, Decode, Encode};
use ockam_core::{Decodable, Encodable, Message, Result};

/// Responder sends back this message with its public UDP IP & PORT and its Address of the
/// Worker dedicated to handle this puncture.
#[derive(Encode, Decode, CborLen, Debug, Clone)]
#[rustfmt::skip]
pub struct UdpPunctureNegotiationMessageInitiate {
    #[n(0)] pub initiator_udp_public_address: String,
    #[n(1)] pub initiator_remote_address: Vec<u8>,
}

/// UDP Puncture negotiation starts with initiator sending this message
/// with its public UDP IP & PORT and an Address of the Worker dedicated to handle this
/// puncture to the `UdpPunctureNegotiationListener` on the responder side via a side-channel.
#[derive(Encode, Decode, CborLen, Debug, Clone)]
#[rustfmt::skip]
pub struct UdpPunctureNegotiationMessageAcknowledge {
    #[n(0)] pub responder_udp_public_address: String,
    #[n(1)] pub responder_remote_address: Vec<u8>,
}

impl Encodable for UdpPunctureNegotiationMessageInitiate {
    fn encode(self) -> Result<Vec<u8>> {
        ockam_core::cbor_encode_preallocate(self)
    }
}

impl Decodable for UdpPunctureNegotiationMessageInitiate {
    fn decode(data: &[u8]) -> Result<Self> {
        Ok(minicbor::decode(data)?)
    }
}

impl Message for UdpPunctureNegotiationMessageInitiate {}

impl Encodable for UdpPunctureNegotiationMessageAcknowledge {
    fn encode(self) -> Result<Vec<u8>> {
        ockam_core::cbor_encode_preallocate(self)
    }
}

impl Decodable for UdpPunctureNegotiationMessageAcknowledge {
    fn decode(data: &[u8]) -> Result<Self> {
        Ok(minicbor::decode(data)?)
    }
}

impl Message for UdpPunctureNegotiationMessageAcknowledge {}
