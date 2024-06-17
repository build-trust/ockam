use minicbor::{CborLen, Decode, Encode};
use ockam_core::{Decodable, Encodable, Message, Result, Route};

/// Internal message type for UDP Puncture
#[derive(Encode, Decode, CborLen, Debug, Clone)]
#[rustfmt::skip]
pub(crate) enum PunctureMessage {
    #[n(0)] Ping,
    #[n(1)] Pong,
    #[n(2)] Payload {
        #[n(0)] onward_route: Route,
        #[n(1)] return_route: Route,
        #[n(2)] payload: Vec<u8>,
    }
}
impl Encodable for PunctureMessage {
    fn encode(self) -> Result<Vec<u8>> {
        ockam_core::cbor_encode_preallocate(self)
    }
}

impl Decodable for PunctureMessage {
    fn decode(data: &[u8]) -> Result<Self> {
        Ok(minicbor::decode(data)?)
    }
}

impl Message for PunctureMessage {}
