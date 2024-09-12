use aya_ebpf::bindings::TC_ACT_PIPE;
use aya_ebpf::macros::map;
use aya_ebpf::maps::HashMap;
use aya_ebpf::programs::TcContext;
use core::cmp::PartialEq;

use aya_log_ebpf::info;

use core::mem;
use network_types::eth::{EthHdr, EtherType};
use network_types::ip::{IpProto, Ipv4Hdr};
use network_types::tcp::TcpHdr;

use crate::conversion::{convert_ockam_to_tcp, convert_tcp_to_ockam};

pub type Proto = u8;

pub type Port = u16;

// TODO: May want to switch to `HashMap::pinned` for efficiency (to share eBPFs)
// TODO: Split Inlet port map into inlet ingress and inlet egress maps for performance
//  (and the same for outlets)

/// Ports that we run inlets on
#[map]
static INLET_PORT_MAP: HashMap<Port, Proto> = HashMap::with_max_entries(1024, 0);

/// Ports that we assigned for currently running connections
#[map]
static OUTLET_PORT_MAP: HashMap<Port, Proto> = HashMap::with_max_entries(1024, 0);

#[derive(PartialEq)]
pub enum Direction {
    Ingress,
    Egress,
}

#[inline(always)]
pub fn try_handle(ctx: TcContext, direction: Direction) -> Result<i32, i32> {
    let ethhdr = ptr_at::<EthHdr>(&ctx, 0).ok_or(TC_ACT_PIPE)?;

    if unsafe { (*ethhdr).ether_type } != EtherType::Ipv4 {
        return Ok(TC_ACT_PIPE);
    }

    let ipv4hdr = ptr_at::<Ipv4Hdr>(&ctx, EthHdr::LEN).ok_or(TC_ACT_PIPE)?;
    let ipv4hdr_stack = unsafe { *ipv4hdr };

    if direction == Direction::Ingress && ipv4hdr_stack.proto == IpProto::Tcp {
        return handle_ingress_tcp_protocol(&ctx, ipv4hdr);
    }

    if direction == Direction::Egress && is_ockam_proto(ipv4hdr_stack.proto as Proto) {
        return handle_egress_ockam_protocol(&ctx, ipv4hdr);
    }

    Ok(TC_ACT_PIPE)
}

#[inline(always)]
fn is_ockam_proto(proto: Proto) -> bool {
    // 146 to 252 are protocol values to be used for custom protocols on top of IPv4.
    // Each ockam node with eBPF portals will generate a random value for itself to minimize risk
    // of intersection with other nodes. Such intersection would not break anything, but decrease
    // performance, as such nodes will receive a copy of packet dedicated for other nodes
    // and discard them.
    // The fact that protocol value is within this range doesn't guarantee that the packet is
    // OCKAM protocol packet, but allows to early skip packets that are definitely not OCKAM
    // protocol
    proto >= 146 && proto <= 252
}

#[inline(always)]
fn handle_ingress_tcp_protocol(ctx: &TcContext, ipv4hdr: *mut Ipv4Hdr) -> Result<i32, i32> {
    let ipv4hdr_stack = unsafe { *ipv4hdr };
    let ipv4hdr_ihl = ipv4hdr_stack.ihl();

    // IPv4 header length must be between 20 and 60 bytes.
    if ipv4hdr_ihl < 5 || ipv4hdr_ihl > 15 {
        return Ok(TC_ACT_PIPE);
    }
    let ipv4hdr_len = ipv4hdr_ihl as usize * 4;

    let src_ip = ipv4hdr_stack.src_addr();
    let dst_ip = ipv4hdr_stack.dst_addr();

    let tcphdr = ptr_at::<TcpHdr>(&ctx, EthHdr::LEN + ipv4hdr_len).ok_or(TC_ACT_PIPE)?;
    let tcphdr_stack = unsafe { *tcphdr };

    let src_port = u16::from_be(tcphdr_stack.source);
    let dst_port = u16::from_be(tcphdr_stack.dest);

    let syn = tcphdr_stack.syn();
    let ack = tcphdr_stack.ack();
    let fin = tcphdr_stack.fin();

    let proto = if let Some(proto) = unsafe { INLET_PORT_MAP.get(&dst_port) } {
        // Inlet logic
        let proto = *proto;
        info!(ctx, "INLET: Converting TCP packet to OCKAM {}", proto);
        proto
    } else if let Some(proto) = unsafe { OUTLET_PORT_MAP.get(&dst_port) } {
        // Outlet logic
        let proto = *proto;
        info!(ctx, "OUTLET: Converting TCP packet to OCKAM {}", proto);
        proto
    } else {
        return Ok(TC_ACT_PIPE);
    };

    info!(
        ctx,
        "TCP PACKET SRC: {}.{}.{}.{}:{}, DST: {}.{}.{}.{}:{}. SYN {} ACK {} FIN {}.",
        src_ip.octets()[0],
        src_ip.octets()[1],
        src_ip.octets()[2],
        src_ip.octets()[3],
        src_port,
        dst_ip.octets()[0],
        dst_ip.octets()[1],
        dst_ip.octets()[2],
        dst_ip.octets()[3],
        dst_port,
        syn,
        ack,
        fin,
    );

    convert_tcp_to_ockam(ctx, ipv4hdr, proto);

    Ok(TC_ACT_PIPE)
}

#[inline(always)]
fn handle_egress_ockam_protocol(ctx: &TcContext, ipv4hdr: *mut Ipv4Hdr) -> Result<i32, i32> {
    let ipv4hdr_stack = unsafe { *ipv4hdr };
    let ipv4hdr_ihl = ipv4hdr_stack.ihl();
    if ipv4hdr_ihl < 5 || ipv4hdr_ihl > 15 {
        return Ok(TC_ACT_PIPE);
    }
    let ipv4hdr_len = ipv4hdr_ihl as usize * 4;

    let src_ip = ipv4hdr_stack.src_addr();
    let dst_ip = ipv4hdr_stack.dst_addr();

    let tcphdr = ptr_at::<TcpHdr>(&ctx, EthHdr::LEN + ipv4hdr_len).ok_or(TC_ACT_PIPE)?;
    let tcphdr_stack = unsafe { *tcphdr };

    let src_port = u16::from_be(tcphdr_stack.source);
    let dst_port = u16::from_be(tcphdr_stack.dest);

    let syn = tcphdr_stack.syn();
    let ack = tcphdr_stack.ack();
    let fin = tcphdr_stack.fin();

    if let Some(port_proto) = unsafe { INLET_PORT_MAP.get(&src_port) } {
        // Inlet logic
        info!(ctx, "INLET: Converting OCKAM {} packet to TCP", *port_proto);
    } else if let Some(port_proto) = unsafe { OUTLET_PORT_MAP.get(&src_port) } {
        // Outlet logic
        info!(
            ctx,
            "OUTLET: Converting OCKAM {} packet to TCP", *port_proto
        );
    } else {
        return Ok(TC_ACT_PIPE);
    }

    info!(
        ctx,
        "TCP PACKET SRC: {}.{}.{}.{}:{}, DST: {}.{}.{}.{}:{}. SYN {} ACK {} FIN {}.",
        src_ip.octets()[0],
        src_ip.octets()[1],
        src_ip.octets()[2],
        src_ip.octets()[3],
        src_port,
        dst_ip.octets()[0],
        dst_ip.octets()[1],
        dst_ip.octets()[2],
        dst_ip.octets()[3],
        dst_port,
        syn,
        ack,
        fin,
    );

    convert_ockam_to_tcp(ctx, ipv4hdr, tcphdr);

    Ok(TC_ACT_PIPE)
}

#[inline(always)]
pub fn ptr_at<T>(ctx: &TcContext, offset: usize) -> Option<*mut T> {
    let start = ctx.data() + offset;
    let end = ctx.data_end();

    if start + mem::size_of::<T>() > end {
        return None;
    }

    Some((start as *mut u8).cast::<T>())
}
