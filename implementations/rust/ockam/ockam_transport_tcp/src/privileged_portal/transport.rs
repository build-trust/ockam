use crate::privileged_portal::{Iface, TcpPacketWriter};
use crate::TcpTransport;
use aya::programs::tc::{qdisc_detach_program, TcAttachType};
use log::{error, info, warn};
use ockam_core::Result;
use std::collections::HashSet;

impl TcpTransport {
    /// Start [`RawSocketProcessor`]. Should be done once.
    pub(crate) async fn start_raw_socket_processor_if_needed(
        &self,
    ) -> Result<Box<dyn TcpPacketWriter>> {
        self.ebpf_support
            .start_raw_socket_processor_if_needed(self.ctx())
            .await
    }

    /// Detach the eBPFs.
    pub fn detach_ebpfs(&self) {
        self.ebpf_support.detach_ebpfs()
    }

    /// Detach all ockam eBPFs from all interfaces for all processes
    pub fn detach_all_ockam_ebpfs_globally() {
        // TODO: Not sure that the best way to do it, but it works.
        info!("Detaching all ebpfs globally");
        let ifaces = match nix::ifaddrs::getifaddrs() {
            Ok(ifaces) => ifaces,
            Err(err) => {
                error!("Error reading network interfaces: {}", err);
                return;
            }
        };

        // Remove duplicates
        let ifaces: HashSet<Iface> = ifaces.into_iter().map(|i| i.interface_name).collect();

        for iface in ifaces {
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
