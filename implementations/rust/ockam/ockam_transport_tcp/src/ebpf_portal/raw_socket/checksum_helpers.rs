use crate::ebpf_portal::packet_binary::tcp_header;
use crate::ebpf_portal::ChecksumAccumulator;
use std::net::Ipv4Addr;

/// Calculate and set checksum for a TCP packet
pub fn tcp_set_checksum(source_ip: Ipv4Addr, destination_ip: Ipv4Addr, packet: &mut [u8]) {
    tcp_header::View::new(&mut packet[..]).check_mut().write(0);

    let checksum = tcp_checksum(source_ip, destination_ip, packet);

    tcp_header::View::new(&mut packet[..])
        .check_mut()
        .write(checksum);
}

fn tcp_checksum(source_ip: Ipv4Addr, destination_ip: Ipv4Addr, packet: &[u8]) -> u16 {
    let mut checksum = ChecksumAccumulator::new();

    let source_ip = source_ip.octets();
    checksum.add_data(&source_ip);

    let destination_ip = destination_ip.octets();
    checksum.add_data(&destination_ip);

    checksum.add_data(&[0, 6]);

    let tcp_length = packet.len() as u16;
    checksum.add_data(&tcp_length.to_be_bytes());
    checksum.add_data(packet.as_ref());

    checksum.finalize()
}

#[cfg(test)]
mod tests {
    use crate::ebpf_portal::raw_socket::checksum_helpers::tcp_checksum;
    use std::net::Ipv4Addr;

    #[test]
    fn tcp_header_ipv4_test() {
        let ipv4_source = Ipv4Addr::new(192, 168, 2, 1);
        let ipv4_destination = Ipv4Addr::new(192, 168, 111, 51);

        let packet = [
            0xc1, 0x67, /* source */
            0x23, 0x28, /* destination */
            0x90, 0x37, 0xd2, 0xb8, /* seq */
            0x94, 0x4b, 0xb2, 0x76, /* ack */
            0x80, 0x18, 0x0f, 0xaf, /* length, flags, win */
            // 0xc0, 0x31, /* checksum */
            0x00, 0x00, /* checksum */
            0x00, 0x00, /* urg ptr */
            0x01, 0x01, /* options: nop */
            0x08, 0x0a, 0x2c, 0x57, 0xcd, 0xa5, 0x02, 0xa0, 0x41, 0x92, /* timestamp */
            0x74, 0x65, 0x73, 0x74, /* "test" */
        ];

        let check = tcp_checksum(ipv4_source, ipv4_destination, &packet);

        assert_eq!(check, 0xc031);
    }
}
