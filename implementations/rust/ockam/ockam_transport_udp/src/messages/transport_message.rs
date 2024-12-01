use crate::messages::RoutingNumber;
use minicbor::{CborLen, Decode, Encode};
use ockam_core::CowBytes;

/// Current protocol version.
pub const CURRENT_VERSION: Version = Version(1);

/// According to IETF RFC 1122 [https://datatracker.ietf.org/doc/html/rfc1122] IP packets of size
/// up to 576 bytes should be supported, which means we can have at least 508 bytes for our
/// payload while using UDP. This should give us high probability of packets not being dropped
/// somewhere on the way.
pub const MAX_ON_THE_WIRE_SIZE: usize = 508;

/// Maximum payload size which will allow message not to exceed [`MAX_ON_THE_WIRE_SIZE`]
/// after encoding.
pub const MAX_PAYLOAD_SIZE: usize = 493;

/// Protocol version.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Encode, Decode, CborLen)]
#[cbor(transparent)]
pub struct Version(#[n(0)] pub u8);

/// UDP transport message type. Used to split [`UdpRoutingMessage`] into UDP datagrams.
///
/// NOTE: Must not be larger than [`MAX_ON_THE_WIRE_SIZE`] bytes when serialized, so the payload
/// max size is [`MAX_PAYLOAD_SIZE`]
#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub struct UdpTransportMessage<'a> {
    #[n(0)] pub version: Version,
    #[n(1)] pub routing_number: RoutingNumber,
    #[n(2)] pub offset: u16,
    #[n(3)] pub total: u16,
    #[b(4)] pub payload: CowBytes<'a>,
}

impl<'a> UdpTransportMessage<'a> {
    /// Constructor.
    pub fn new(
        version: Version,
        routing_number: RoutingNumber,
        offset: u16,
        total: u16,
        payload: impl Into<CowBytes<'a>>,
    ) -> Self {
        Self {
            version,
            routing_number,
            offset,
            total,
            payload: payload.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::messages::{
        RoutingNumber, UdpTransportMessage, Version, MAX_ON_THE_WIRE_SIZE, MAX_PAYLOAD_SIZE,
    };

    #[test]
    fn test_max_size_current_protocol() {
        let msg = UdpTransportMessage::new(
            Version(u8::MAX),
            RoutingNumber(u16::MAX),
            u16::MAX,
            u16::MAX,
            vec![0u8; MAX_PAYLOAD_SIZE],
        );

        let len = ockam_core::cbor_encode_preallocate(msg).unwrap().len();

        assert!(len <= MAX_ON_THE_WIRE_SIZE);
    }

    #[test]
    fn test_max_size_max_protocol() {
        let msg = UdpTransportMessage::new(
            Version(u8::MAX),
            RoutingNumber(u16::MAX),
            u16::MAX,
            u16::MAX,
            vec![0u8; MAX_PAYLOAD_SIZE],
        );

        let len = ockam_core::cbor_encode_preallocate(msg).unwrap().len();

        assert_eq!(len, MAX_ON_THE_WIRE_SIZE);
    }
}
