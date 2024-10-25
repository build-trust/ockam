use crate::ebpf_portal::packet_binary::{ipv4_header, stripped_tcp_header, tcp_header};
use crate::ebpf_portal::Port;
use std::net::Ipv4Addr;

/// Result of reading packet from RawSocket
pub struct RawSocketReadResult {
    /// Info from IPv4 header
    pub ipv4_info: Ipv4Info,
    /// Info from TCP header
    pub tcp_info: TcpInfo,
    /// Part of the TCP header (without ports) and TCP payload
    pub header_and_payload: TcpStrippedHeaderAndPayload,
}

/// TCP Header excluding first 4 bytes (src and dst ports) + payload
pub struct TcpStrippedHeaderAndPayload(Vec<u8>);

impl TcpStrippedHeaderAndPayload {
    /// Constructor
    pub fn new(bytes: Vec<u8>) -> Option<Self> {
        if bytes.len() < 16 {
            return None;
        }

        Some(Self(bytes))
    }

    /// Consume and return the data
    pub fn take(self) -> Vec<u8> {
        self.0
    }

    /// Length
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns true if SYN flag is set and all other flags are unset
    pub fn is_syn_only(&self) -> bool {
        let view = stripped_tcp_header::View::new(self.0.as_slice());

        let flags = view.flags().read();

        flags == 0b00000010
    }
}

/// IPv4 header, but only required info is parsed
pub struct Ipv4Info {
    source_ip: Ipv4Addr,
    destination_ip: Ipv4Addr,
    header_length: u8,
    tot_len: u16,
}

impl<S: AsRef<[u8]>> From<&ipv4_header::View<S>> for Ipv4Info {
    fn from(view: &ipv4_header::View<S>) -> Self {
        let source_ip = Ipv4Addr::from(view.src_addr().read());
        let destination_ip = Ipv4Addr::from(view.dst_addr().read());

        let version_and_ihl = view.version_and_ihl().read();

        let header_length = (version_and_ihl & 0b00001111) << 2;

        let tot_len = view.tot_len().read();

        Self::new(source_ip, destination_ip, header_length, tot_len)
    }
}

impl Ipv4Info {
    /// Constructor
    pub fn new(
        source_ip: Ipv4Addr,
        destination_ip: Ipv4Addr,
        header_length: u8,
        tot_len: u16,
    ) -> Self {
        Self {
            source_ip,
            destination_ip,
            header_length,
            tot_len,
        }
    }

    /// Minimum length (without options)
    pub const HEADER_BASE_LEN: usize = 20;

    /// Header length
    pub fn header_length(&self) -> u8 {
        self.header_length
    }

    /// Total length
    pub fn total_length(&self) -> u16 {
        self.tot_len
    }

    /// Source IP
    pub fn source_ip(&self) -> Ipv4Addr {
        self.source_ip
    }

    /// Destination IP
    pub fn destination_ip(&self) -> Ipv4Addr {
        self.destination_ip
    }
}

/// TCP header, but only required info is parsed
pub struct TcpInfo {
    source_port: Port,
    destination_port: Port,
    header_length: u8,
    flags: u8,
}

impl<S: AsRef<[u8]>> From<&tcp_header::View<S>> for TcpInfo {
    fn from(view: &tcp_header::View<S>) -> Self {
        let source_port = view.source().read();
        let destination_port = view.dest().read();
        let flags = view.flags().read();

        let header_length = (view.doff_and_reserved().read() & 0b11110000) >> 2;

        Self::new(source_port, destination_port, header_length, flags)
    }
}

impl TcpInfo {
    /// Minimum length (without options)
    pub const HEADER_BASE_LEN: usize = 20;

    /// Constructor
    pub fn new(source_port: Port, destination_port: Port, header_length: u8, flags: u8) -> Self {
        Self {
            source_port,
            destination_port,
            header_length,
            flags,
        }
    }

    /// Header length
    pub fn header_length(&self) -> u8 {
        self.header_length
    }

    /// Source port
    pub fn source_port(&self) -> Port {
        self.source_port
    }

    /// Destination port
    pub fn destination_port(&self) -> Port {
        self.destination_port
    }

    /// TCP flags
    pub fn flags(&self) -> u8 {
        self.flags
    }

    /// FIN flag
    pub fn fin(&self) -> bool {
        self.flags & 0b00000001 != 0
    }

    /// SYN flag
    pub fn syn(&self) -> bool {
        self.flags & 0b00000010 != 0
    }

    /// RST flag
    pub fn rst(&self) -> bool {
        self.flags & 0b00000100 != 0
    }

    /// ACK flag
    pub fn ack(&self) -> bool {
        self.flags & 0b00001000 != 0
    }

    /// Returns true if SYN flag is set and all other flags are unset
    pub fn is_syn_only(&self) -> bool {
        self.flags == 0b00000010
    }
}
