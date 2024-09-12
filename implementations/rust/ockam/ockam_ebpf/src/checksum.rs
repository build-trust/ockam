use crate::checksum_helpers::{checksum, checksum_update_word};
use aya_ebpf::programs::TcContext;
use network_types::ip::Ipv4Hdr;
use network_types::tcp::TcpHdr;

#[inline(always)]
pub fn iph_update_csum(ctx: &TcContext, ipv4hdr: *mut Ipv4Hdr) {
    unsafe {
        let len = (*ipv4hdr).ihl() as usize * 4;

        (*ipv4hdr).check = 0;

        let check = checksum(ipv4hdr as usize, len, ctx.data_end());

        (*ipv4hdr).check = check;
    }
}

#[inline(always)]
pub fn tcph_update_csum(ipv4hdr: *const Ipv4Hdr, tcphdr: *mut TcpHdr) {
    // TODO: Theoretically, removing all big endian conversions will yield the same result.

    unsafe {
        // User-space code calculates checksum using 0.0.0.0 as src IP,  because it's not known
        // at that moment. Here we will update the checksum in respect to the actual src IP value.
        let original_check = u16::from_be((*tcphdr).check);

        let actual_ip = (*ipv4hdr).src_addr;
        let actual_ip_word1 = u16::from_be((actual_ip & 0xffff) as u16);
        let actual_ip_word2 = u16::from_be((actual_ip >> 16) as u16);

        let check = checksum_update_word(original_check, 0, actual_ip_word1);
        let check = checksum_update_word(check, 0, actual_ip_word2);

        (*tcphdr).check = check.to_be();
    }
}
