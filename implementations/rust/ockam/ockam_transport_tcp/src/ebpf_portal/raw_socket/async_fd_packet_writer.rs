use crate::ebpf_portal::packet::TcpStrippedHeaderAndPayload;
use crate::ebpf_portal::packet_binary::tcp_header_ports;
use crate::ebpf_portal::{tcp_set_checksum, Port, TcpPacketWriter};
use async_trait::async_trait;
use log::{debug, error};
use nix::sys::socket::{MsgFlags, SockaddrIn};
use ockam_core::Result;
use ockam_transport_core::TransportError;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::Arc;
use tokio::io::unix::AsyncFd;
use tokio::io::Interest;

/// RawSocket packet writer implemented via tokio's AsyncFd
pub struct AsyncFdPacketWriter {
    fd: Arc<AsyncFd<OwnedFd>>,
}

impl AsyncFdPacketWriter {
    /// Constructor
    pub fn new(fd: Arc<AsyncFd<OwnedFd>>) -> Self {
        Self { fd }
    }
}

#[async_trait]
impl TcpPacketWriter for AsyncFdPacketWriter {
    async fn write_packet(
        &self,
        src_port: Port,
        destination_ip: Ipv4Addr,
        dst_port: Port,
        header_and_payload: TcpStrippedHeaderAndPayload,
    ) -> Result<()> {
        // We need to prepend ports to the beginning of the header, instead of cloning, let's
        // add this data to the end and reverse the whole binary few types for the same result,
        // but should be more efficient
        let mut packet = header_and_payload.take();
        packet.reserve(4);
        packet.reverse();

        let mut ports = [0u8; 4];
        let mut ports_view = tcp_header_ports::View::new(&mut ports);
        ports_view.source_mut().write(src_port);
        ports_view.dest_mut().write(dst_port);
        ports.reverse();

        packet.extend_from_slice(&ports[..]);
        packet.reverse();

        tcp_set_checksum(Ipv4Addr::UNSPECIFIED, destination_ip, &mut packet);

        let destination_addr = SockaddrIn::from(SocketAddrV4::new(destination_ip, 0));

        enum WriteResult {
            Ok { len: usize },
            Err(TransportError),
        }

        let write_result = self
            .fd
            .async_io(Interest::WRITABLE, |fd| {
                let res = nix::sys::socket::sendto(
                    fd.as_raw_fd(),
                    packet.as_slice(),
                    &destination_addr,
                    MsgFlags::empty(),
                );

                match res {
                    Ok(len) => Ok(WriteResult::Ok { len }),
                    Err(err) => {
                        if err == nix::errno::Errno::EWOULDBLOCK {
                            debug!("RawSocket write would block");
                            return Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, err));
                        }
                        error!("Failed to write packet. Err: {}", err);
                        Ok(WriteResult::Err(TransportError::RawSocketWrite(
                            err.to_string(),
                        )))
                    }
                }
            })
            .await
            .map_err(|err| TransportError::RawSocketWrite(err.to_string()))?;

        let len = match write_result {
            WriteResult::Ok { len } => len,
            WriteResult::Err(err) => return Err(err)?,
        };

        if len != packet.len() {
            return Err(TransportError::RawSocketWrite(format!(
                "Could not write the whole packet. Packet len: {}. Actually written: {}",
                packet.len(),
                len
            )))?;
        }

        Ok(())
    }
}
