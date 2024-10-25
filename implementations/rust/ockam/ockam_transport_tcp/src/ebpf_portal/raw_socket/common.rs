use crate::ebpf_portal::packet::TcpStrippedHeaderAndPayload;
use minicbor::{Decode, Encode};
use rand::distributions::{Distribution, Standard};
use rand::Rng;

/// Network interface name
pub type Iface = String;

/// Port. Should be always in native endianness, though we don't introduce a separate type for
/// compiler to check that guarantee.
pub type Port = u16;

/// IP Protocol
pub type Proto = u8;

/// Unique random connection identifier
#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
#[cbor(transparent)]
#[rustfmt::skip]
pub struct ConnectionIdentifier(#[n(0)] u64);

impl Distribution<ConnectionIdentifier> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ConnectionIdentifier {
        ConnectionIdentifier(rng.gen())
    }
}

/// Packet exchanged between the Inlet and the Outlet
#[derive(Encode, Decode)]
#[rustfmt::skip]
pub struct OckamPortalPacket {
    /// Unique TCP connection identifier
    #[n(0)] pub connection_identifier: ConnectionIdentifier,
    /// Monotonic increasing route numeration
    #[n(1)] pub route_index: u32,
    /// Stripped (without ports) TCP header and payload
    #[n(2)] pub header_and_payload: Vec<u8>,
}

impl OckamPortalPacket {
    /// Dissolve into parts consuming the original value to avoid clones
    pub fn dissolve(self) -> Option<(ConnectionIdentifier, u32, TcpStrippedHeaderAndPayload)> {
        let header_and_payload = TcpStrippedHeaderAndPayload::new(self.header_and_payload)?;

        Some((
            self.connection_identifier,
            self.route_index,
            header_and_payload,
        ))
    }

    /// Create from packet
    pub fn from_tcp_packet(
        connection_identifier: ConnectionIdentifier,
        route_index: u32,
        header_and_payload: TcpStrippedHeaderAndPayload,
    ) -> Self {
        Self {
            connection_identifier,
            route_index,
            header_and_payload: header_and_payload.take(),
        }
    }
}
