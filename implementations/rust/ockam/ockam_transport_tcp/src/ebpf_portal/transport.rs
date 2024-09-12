use crate::ebpf_portal::Iface;
use crate::TcpTransport;
use ockam_core::Result;
use pnet::transport::TransportSender;
use std::sync::{Arc, RwLock};

impl TcpTransport {
    /// Start [`RawSocketProcessor`]. Should be done once.
    pub(crate) async fn start_raw_socket_processor_if_needed(
        &self,
    ) -> Result<Arc<RwLock<TransportSender>>> {
        self.ebpf_support
            .start_raw_socket_processor_if_needed(self.ctx())
            .await
    }

    // TODO: eBPF Should we dispatch it to the sync thread?
    pub(crate) fn attach_ebpf_if_needed(&self, iface: Iface) -> Result<()> {
        self.ebpf_support.attach_ebpf_if_needed(iface)
    }

    /// Detach the eBPFs.
    pub fn detach_ebpfs(&self) {
        self.ebpf_support.detach_ebpfs()
    }
}

impl Drop for TcpTransport {
    fn drop(&mut self) {
        self.detach_ebpfs()
    }
}
