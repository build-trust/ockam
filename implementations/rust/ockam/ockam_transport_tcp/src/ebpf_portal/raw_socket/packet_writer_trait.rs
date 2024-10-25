use crate::ebpf_portal::packet::TcpStrippedHeaderAndPayload;
use crate::ebpf_portal::Port;
use async_trait::async_trait;
use ockam_core::Result;
use std::net::Ipv4Addr;

/// Trait for writing packets to the RawSocket
#[async_trait]
pub trait TcpPacketWriter: Send + Sync + 'static {
    /// Write packet to the RawSocket
    async fn write_packet(
        &self,
        src_port: Port,
        destination_ip: Ipv4Addr,
        dst_port: Port,
        header_and_payload: TcpStrippedHeaderAndPayload,
    ) -> Result<()>;
}
