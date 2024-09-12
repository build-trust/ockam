use crate::checksum::{iph_update_csum, tcph_update_csum};
use crate::common::Proto;
use aya_ebpf::programs::TcContext;
use core::mem::offset_of;
use core::ptr::copy_nonoverlapping;
use network_types::ip::{IpProto, Ipv4Hdr};
use network_types::tcp::TcpHdr;

pub fn convert_tcp_to_ockam(ctx: &TcContext, ipv4hdr: *mut Ipv4Hdr, ockam_proto: Proto) {
    unsafe {
        // Basically ipv4hdr.proto = ockam_proto, that can't be done cause type-safety
        let proto_ptr = (&raw const ockam_proto).cast::<u8>();
        let hdr_proto_ptr = ipv4hdr.cast::<u8>().add(offset_of!(Ipv4Hdr, proto));

        copy_nonoverlapping(proto_ptr, hdr_proto_ptr, size_of::<Proto>());
    }

    iph_update_csum(ctx, ipv4hdr);
}

pub fn convert_ockam_to_tcp(ctx: &TcContext, ipv4hdr: *mut Ipv4Hdr, tcphdr: *mut TcpHdr) {
    unsafe {
        (*ipv4hdr).proto = IpProto::Tcp;
    }

    iph_update_csum(ctx, ipv4hdr);
    tcph_update_csum(ipv4hdr, tcphdr);
}
