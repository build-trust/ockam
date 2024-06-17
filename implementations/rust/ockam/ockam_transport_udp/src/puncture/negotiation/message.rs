use minicbor::{CborLen, Decode, Encode};
use ockam_core::{Decodable, Encodable, Message, Result};

#[derive(Encode, Decode, CborLen, Debug, Clone)]
#[rustfmt::skip]
pub(crate) enum UdpPunctureNegotiationMessage {
    /// UDP Puncture negotiation starts with initiator sending this message
    /// with its public UDP IP & PORT and an Address of the Worker dedicated to handle this
    /// puncture to the `UdpPunctureNegotiationListener` on the responder side via a side-channel.
    #[n(0)] Initiate {
        #[n(0)] initiator_udp_public_address: String,
        #[n(1)] initiator_remote_address: Vec<u8>,
    },
    /// Responder sends back this message with its public UDP IP & PORT and its Address of the
    /// Worker dedicated to handle this puncture.
    #[n(1)] Acknowledge {
        #[n(0)] responder_udp_public_address: String,
        #[n(1)] responder_remote_address: Vec<u8>,
    }
}

impl Encodable for UdpPunctureNegotiationMessage {
    fn encode(self) -> Result<Vec<u8>> {
        ockam_core::cbor_encode_preallocate(self)
    }
}

impl Decodable for UdpPunctureNegotiationMessage {
    fn decode(data: &[u8]) -> Result<Self> {
        Ok(minicbor::decode(data)?)
    }
}

impl Message for UdpPunctureNegotiationMessage {}
