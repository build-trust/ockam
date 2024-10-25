use binary_layout::binary_layout;

binary_layout!(tcp_header, BigEndian, {
    source: u16,
    dest: u16,
    seq: u32,
    ack_seq: u32,
    doff_and_reserved: u8,
    flags: u8,
    window: u16,
    check: u16,
    urg_ptr: u16,
});

binary_layout!(tcp_header_ports, BigEndian, {
    source: u16,
    dest: u16,
});

binary_layout!(stripped_tcp_header, BigEndian, {
    seq: u32,
    ack_seq: u32,
    doff_and_reserved: u8,
    flags: u8,
    window: u16,
    check: u16,
    urg_ptr: u16,
});

binary_layout!(ipv4_header, BigEndian, {
    version_and_ihl: u8,
    dscp_and_ecn: u8,
    tot_len: u16,
    id: u16,
    flags_and_frag_off: u16,
    ttl: u8,
    proto: u8,
    check: u16,
    src_addr: u32,
    dst_addr: u32,
});
