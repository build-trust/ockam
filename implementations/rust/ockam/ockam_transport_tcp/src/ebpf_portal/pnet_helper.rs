use core::net::IpAddr;
use core::{mem, net};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::transport::TransportChannelType::{Layer3, Layer4};
use pnet::transport::{TransportProtocol, TransportReceiver};
use std::mem::size_of;

pub fn next_tcp_packet(receiver: &mut TransportReceiver) -> std::io::Result<(TcpPacket, IpAddr)> {
    let mut caddr: pnet_sys::SockAddrStorage = unsafe { mem::zeroed() };
    let res = pnet_sys::recv_from(receiver.socket.fd, &mut receiver.buffer[..], &mut caddr);

    let offset = match receiver.channel_type {
        Layer4(TransportProtocol::Ipv4(_)) => {
            let ip_header = Ipv4Packet::new(&receiver.buffer[..]).unwrap();

            ip_header.get_header_length() as usize * 4usize
        }
        Layer3(_) => {
            fixup_packet(&mut receiver.buffer[..]);

            0
        }
        _ => 0,
    };

    match res {
        Ok(len) => {
            // FIXME: Is that guaranteed that we receive the packet fully in one read?
            let packet = TcpPacket::new(&receiver.buffer[offset..len]).unwrap();
            let addr = pnet_sys::sockaddr_to_addr(&caddr, size_of::<pnet_sys::SockAddrStorage>());
            let ip = match addr.unwrap() {
                net::SocketAddr::V4(sa) => IpAddr::V4(*sa.ip()),
                net::SocketAddr::V6(sa) => IpAddr::V6(*sa.ip()),
            };
            Ok((packet, ip))
        }
        Err(e) => Err(e),
    }
}

#[cfg(any(
    target_os = "freebsd",
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos"
))]
fn fixup_packet(buffer: &mut [u8]) {
    use pnet::pnet_packet::ipv4::MutableIpv4Packet;

    let buflen = buffer.len();
    let mut new_packet = MutableIpv4Packet::new(buffer).unwrap();

    let length = u16::from_be(new_packet.get_total_length());
    new_packet.set_total_length(length);

    // OS X does this awesome thing where it removes the header length
    // from the total length sometimes.
    let length =
        new_packet.get_total_length() as usize + (new_packet.get_header_length() as usize * 4usize);
    if length == buflen {
        new_packet.set_total_length(length as u16)
    }

    let offset = u16::from_be(new_packet.get_fragment_offset());
    new_packet.set_fragment_offset(offset);
}

#[cfg(all(
    not(target_os = "freebsd"),
    not(any(target_os = "macos", target_os = "ios", target_os = "tvos"))
))]
fn fixup_packet(_buffer: &mut [u8]) {}
