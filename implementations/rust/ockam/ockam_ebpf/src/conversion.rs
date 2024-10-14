use crate::checksum::{iph_update_csum, tcph_update_csum};
use crate::common::Proto;
use aya_ebpf::programs::TcContext;
use network_types::ip::{IpProto, Ipv4Hdr};
use network_types::tcp::TcpHdr;

#[inline(always)]
pub fn convert_tcp_to_ockam(ctx: &TcContext, ipv4hdr: *mut Ipv4Hdr, ockam_proto: Proto) {
    unsafe {
        (*ipv4hdr).proto = core::mem::transmute(ockam_proto);
    }

    iph_update_csum(ctx, ipv4hdr);
}

#[inline(always)]
pub fn convert_ockam_to_tcp(ctx: &TcContext, ipv4hdr: *mut Ipv4Hdr, tcphdr: *mut TcpHdr) {
    unsafe {
        (*ipv4hdr).proto = IpProto::Tcp;
    }

    iph_update_csum(ctx, ipv4hdr);
    tcph_update_csum(ipv4hdr, tcphdr);
}
