use minicbor::{Decode, Encode};
use ockam_core::CowBytes;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::Packet;
use std::net::Ipv4Addr;

#[allow(missing_docs)]
#[derive(Encode, Decode)]
#[rustfmt::skip]
pub struct OckamPortalPacket<'a> {
    #[n(0)] pub sequence: u32,
    #[n(1)] pub acknowledgement: u32,
    #[n(2)] pub data_offset: u8,
    #[n(3)] pub reserved: u8,
    #[n(4)] pub flags: u8,
    #[n(5)] pub window: u16,
    #[n(6)] pub urgent_ptr: u16,
    #[n(7)] pub options: Vec<TcpOption>,
    #[b(8)] pub payload: CowBytes<'a>,
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

impl OckamPortalPacket<'_> {
    /// Clone data to make an owned version of an instance.
    pub fn into_owned(self) -> OckamPortalPacket<'static> {
        OckamPortalPacket {
            sequence: self.sequence,
            acknowledgement: self.acknowledgement,
            data_offset: self.data_offset,
            reserved: self.reserved,
            flags: self.flags,
            window: self.window,
            urgent_ptr: self.urgent_ptr,
            options: self.options,
            payload: self.payload.to_owned(),
        }
    }
}

impl From<RawSocketMessage> for OckamPortalPacket<'_> {
    fn from(value: RawSocketMessage) -> Self {
        Self {
            sequence: value.sequence,
            acknowledgement: value.acknowledgement,
            data_offset: value.data_offset,
            reserved: value.reserved,
            flags: value.flags,
            window: value.window,
            urgent_ptr: value.urgent_ptr,
            options: value.options.into_iter().map(Into::into).collect(),
            payload: value.payload.into(),
        }
    }
}

#[allow(missing_docs)]
pub struct RawSocketMessage {
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
impl RawSocketMessage {
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
