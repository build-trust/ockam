use minicbor::{Decode, Encode};
use pnet::packet::tcp::TcpPacket;
use pnet::packet::Packet;
use rand::distributions::{Distribution, Standard};
use rand::Rng;
use std::net::Ipv4Addr;

/// Port
pub type Port = u16;

/// Network interface name
pub type Iface = String;

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

#[allow(missing_docs)]
#[derive(Encode, Decode)]
#[rustfmt::skip]
pub struct OckamPortalPacket {
    #[n(0)] pub connection_identifier: ConnectionIdentifier,
    #[n(1)] pub route_index: u32,
    #[n(2)] pub sequence: u32,
    #[n(3)] pub acknowledgement: u32,
    #[n(4)] pub data_offset: u8,
    #[n(5)] pub reserved: u8,
    #[n(6)] pub flags: u8,
    #[n(7)] pub window: u16,
    #[n(8)] pub urgent_ptr: u16,
    #[n(9)] pub options: Vec<TcpOption>,
    #[n(10)] pub payload: Vec<u8>,
}

#[allow(missing_docs)]
#[derive(Encode, Decode)]
#[rustfmt::skip]
pub struct TcpOption {
    #[n(0)] pub kind: u8,
    #[n(1)] pub length: Vec<u8>,
    #[n(2)] pub data: Vec<u8>,
}

impl From<TcpOption> for pnet::packet::tcp::TcpOption {
    fn from(value: TcpOption) -> Self {
        Self {
            number: pnet::packet::tcp::TcpOptionNumber(value.kind),
            length: value.length,
            data: value.data,
        }
    }
}

impl OckamPortalPacket {
    /// Transform
    pub fn from_raw_socket_packet(
        value: RawSocketPacket,
        connection_identifier: ConnectionIdentifier,
        route_index: u32,
    ) -> Self {
        Self {
            connection_identifier,
            route_index,
            sequence: value.sequence,
            acknowledgement: value.acknowledgement,
            data_offset: value.data_offset,
            reserved: value.reserved,
            flags: value.flags,
            window: value.window,
            urgent_ptr: value.urgent_ptr,
            options: value.options.into_iter().map(Into::into).collect(),
            payload: value.payload,
        }
    }
}

#[allow(missing_docs)]
pub struct RawSocketPacket {
    pub source_ip: Ipv4Addr,

    pub source: u16,
    pub destination: u16,
    pub sequence: u32,
    pub acknowledgement: u32,
    pub data_offset: u8,
    pub reserved: u8,
    pub flags: u8,
    pub window: u16,
    pub checksum: u16,
    pub urgent_ptr: u16,
    pub options: Vec<RawTcpOption>,
    pub payload: Vec<u8>,
}

#[allow(missing_docs)]
pub struct ParsedRawSocketPacket {
    pub packet: RawSocketPacket,

    pub destination_ip: Ipv4Addr,
    pub destination_port: Port,
}

impl From<pnet::packet::tcp::TcpOption> for RawTcpOption {
    fn from(value: pnet::packet::tcp::TcpOption) -> Self {
        Self {
            kind: value.number.0,
            length: value.length,
            data: value.data,
        }
    }
}

impl From<RawTcpOption> for TcpOption {
    fn from(value: RawTcpOption) -> Self {
        Self {
            kind: value.kind,
            length: value.length,
            data: value.data,
        }
    }
}

#[allow(missing_docs)]
pub struct RawTcpOption {
    pub kind: u8,
    pub length: Vec<u8>,
    pub data: Vec<u8>,
}

#[allow(missing_docs)]
impl RawSocketPacket {
    pub fn from_packet(packet: TcpPacket<'_>, source_ip: Ipv4Addr) -> Self {
        Self {
            source_ip,
            source: packet.get_source(),
            destination: packet.get_destination(),
            sequence: packet.get_sequence(),
            acknowledgement: packet.get_acknowledgement(),
            data_offset: packet.get_data_offset(),
            reserved: packet.get_reserved(),
            flags: packet.get_flags(),
            window: packet.get_window(),
            checksum: packet.get_checksum(),
            urgent_ptr: packet.get_urgent_ptr(),
            options: packet.get_options().into_iter().map(Into::into).collect(),
            payload: packet.payload().to_vec(),
        }
    }
}
