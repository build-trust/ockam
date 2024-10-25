use crate::ebpf_portal::packet::RawSocketReadResult;
use async_trait::async_trait;
use ockam_core::Result;

/// Trait for reading packets from the RawSocket
#[async_trait]
pub trait TcpPacketReader: Send + Sync + 'static {
    /// Read packet from the RawSocket
    async fn read_packet(&mut self) -> Result<RawSocketReadResult>;
}
