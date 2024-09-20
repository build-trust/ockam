use crate::ebpf_portal::Iface;
use crate::TcpTransport;
use aya::programs::tc::{qdisc_detach_program, TcAttachType};
use log::{info, warn};
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

    /// List all interfaces with defined IPv4 address
    pub fn all_interfaces_with_address() -> Vec<String> {
        let ifaddrs = nix::ifaddrs::getifaddrs().unwrap(); // FIXME
        ifaddrs
            .filter_map(|ifaddr| {
                let addr = match ifaddr.address {
                    Some(addr) => addr,
                    None => return None,
                };

                addr.as_sockaddr_in()?;

                Some(ifaddr.interface_name)
            })
            .collect::<Vec<_>>()
    }

    /// Detach all ockam eBPFs from all interfaces for all processes
    pub fn detach_all_ockam_ebpfs_globally() {
        // TODO: Not sure that the best way to do it, but it works.
        info!("Detaching all ebpfs globally");
        for iface in Self::all_interfaces_with_address() {
            match qdisc_detach_program(&iface, TcAttachType::Ingress, "ockam_ingress") {
                Ok(_) => {
                    info!("Detached ockam_ingress from {}", iface);
                }
                Err(err) => {
                    warn!(
                        "Could not detach ockam_ingress from {}. Error {}",
                        iface, err
                    );
                }
            }
            match qdisc_detach_program(&iface, TcAttachType::Egress, "ockam_egress") {
                Ok(_) => {
                    info!("Detached ockam_egress from {}", iface);
                }
                Err(err) => {
                    warn!(
                        "Could not detach ockam_egress from {}. Error {}",
                        iface, err
                    );
                }
            }
        }
    }
}

impl Drop for TcpTransport {
    fn drop(&mut self) {
        self.detach_ebpfs()
    }
}
