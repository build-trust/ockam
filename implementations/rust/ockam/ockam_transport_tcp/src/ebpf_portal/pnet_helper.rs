use core::{mem, net};
use ockam_core::Result;
use ockam_transport_core::TransportError;
use pnet::packet::ip::IpNextHeaderProtocol;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::transport;
use pnet::transport::{
    TransportChannelType, TransportProtocol, TransportReceiver, TransportSender,
};
use std::mem::size_of;
use std::net::Ipv4Addr;

pub fn create_raw_socket(ip_proto: u8) -> Result<(TransportSender, TransportReceiver)> {
    Ok(transport::transport_channel(
        1024 * 1024, // FIXME
        TransportChannelType::Layer4(TransportProtocol::Ipv4(IpNextHeaderProtocol::new(ip_proto))),
    )
    .map_err(|_| TransportError::RawSocketCreationError)?)
}

pub fn next_tcp_packet(
    receiver: &mut TransportReceiver,
) -> std::io::Result<(TcpPacket, Ipv4Addr, Ipv4Addr)> {
    loop {
        let mut caddr: pnet_sys::SockAddrStorage = unsafe { mem::zeroed() };
        let len = pnet_sys::recv_from(receiver.socket.fd, &mut receiver.buffer[..], &mut caddr)?;

        let src = pnet_sys::sockaddr_to_addr(&caddr, size_of::<pnet_sys::SockAddrStorage>())?;
        let src = match src {
            net::SocketAddr::V4(sa) => *sa.ip(),
            net::SocketAddr::V6(_) => continue,
        };

        // FIXME: Is that guaranteed that we receive the packet fully in one read?
        let ip_header = Ipv4Packet::new(&receiver.buffer[..]).unwrap();
        let offset = ip_header.get_header_length() as usize * 4usize;

        let dst = ip_header.get_destination();

        let packet = TcpPacket::new(&receiver.buffer[offset..len]).unwrap();

        return Ok((packet, src, dst));
    }
}
