use ockam_core::env::get_env;

/// Various vector sizes that affect performance and robustness of UDP transport
#[derive(Clone, Copy, Debug)]
pub struct UdpSizeOptions {
    /// Number of pending messages per peer
    pub pending_messages_per_peer: u16,
    /// Max size of the UDP packet on the wire
    pub max_on_the_wire_packet_size: usize,
    /// Max payload size for a UDP packet
    /// A packet with payload of this size should increase to max_on_the_wire_size after encoding
    pub max_payload_size_per_packet: usize,
}

impl Default for UdpSizeOptions {
    fn default() -> Self {
        // According to IETF RFC 1122 [https://datatracker.ietf.org/doc/html/rfc1122] IP packets
        // of size up to 576 bytes should be supported. This should give us high probability of packets
        // not being dropped somewhere on the way.
        let default_on_the_wire_size = 508usize;

        Self {
            pending_messages_per_peer: 5,
            max_on_the_wire_packet_size: default_on_the_wire_size,
            max_payload_size_per_packet: Self::calculate_max_payload_size_per_packet(
                default_on_the_wire_size,
            ),
        }
    }
}

impl UdpSizeOptions {
    fn calculate_max_payload_size_per_packet(max_on_the_wire_size: usize) -> usize {
        // Encoding overhead for [`UdpTransportMessage`]
        let encoding_overhead = 15usize;
        max_on_the_wire_size - encoding_overhead
    }

    /// Read values from environment with fallback to default values
    pub fn read_from_env() -> Self {
        let mut s = Self::default();

        if let Some(p) = get_env("OCKAM_UDP_PENDING_MESSAGES_PER_PEER")
            .ok()
            .flatten()
        {
            s.pending_messages_per_peer = p;
        }

        if let Some(p) = get_env("OCKAM_UDP_MAX_ON_THE_WIRE_PACKET_SIZE")
            .ok()
            .flatten()
        {
            s.max_on_the_wire_packet_size = p;
            s.max_payload_size_per_packet = Self::calculate_max_payload_size_per_packet(p);
        }

        s
    }
}

#[allow(non_snake_case)]
#[test]
fn udp_size_options_read_from_env__env_var_set__pulls_correct_value() {
    let m: u16 = rand::random();
    std::env::set_var("OCKAM_UDP_PENDING_MESSAGES_PER_PEER", m.to_string());
    std::env::set_var("OCKAM_UDP_MAX_ON_THE_WIRE_PACKET_SIZE", 800.to_string());

    let size_options = UdpSizeOptions::read_from_env();

    assert_eq!(size_options.pending_messages_per_peer, m);
    assert_eq!(size_options.max_on_the_wire_packet_size, 800);
    assert_eq!(size_options.max_payload_size_per_packet, 785);
}
