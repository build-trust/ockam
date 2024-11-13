use crate::privileged_portal::packet::TcpStrippedHeaderAndPayload;
use crate::privileged_portal::packet_binary::tcp_header_ports;
use crate::privileged_portal::{tcp_set_checksum, Port, TcpPacketWriter};
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
    // The idea is that each writer has its own buffer, so that we avoid either
    //  1. Waiting for the lock on a shared buffer
    //  2. Allocating new buffer on every write operations
    buffer: Vec<u8>,
    fd: Arc<AsyncFd<OwnedFd>>,
}

impl AsyncFdPacketWriter {
    /// Constructor
    pub fn new(fd: Arc<AsyncFd<OwnedFd>>) -> Self {
        Self { buffer: vec![], fd }
    }
}

#[async_trait]
impl TcpPacketWriter for AsyncFdPacketWriter {
    async fn write_packet(
        &mut self,
        src_port: Port,
        dst_ip: Ipv4Addr,
        dst_port: Port,
        header_and_payload: TcpStrippedHeaderAndPayload<'_>,
    ) -> Result<()> {
        self.buffer.clear();
        self.buffer.reserve(header_and_payload.len() + 4);

        let mut ports = [0u8; 4];
        let mut ports_view = tcp_header_ports::View::new(&mut ports);
        ports_view.source_mut().write(src_port);
        ports_view.dest_mut().write(dst_port);

        self.buffer.extend_from_slice(ports.as_slice());
        self.buffer.extend_from_slice(header_and_payload.as_slice());

        tcp_set_checksum(Ipv4Addr::UNSPECIFIED, dst_ip, &mut self.buffer);

        let destination_addr = SockaddrIn::from(SocketAddrV4::new(dst_ip, 0));

        // We don't pick source IP, kernel does it for us by performing Routing Table lookup.
        // The problem is that if for some reason tcp packets from one connection
        // use different src_ip, the connection would be disrupted.
        // As an alternative, we could build IPv4 header ourselves and control it by setting
        // IP_HDRINCL socket option, but that brings a lot of challenges.

        enum WriteResult {
            Ok { len: usize },
            Err(TransportError),
        }

        let write_result = self
            .fd
            .async_io(Interest::WRITABLE, |fd| {
                let res = nix::sys::socket::sendto(
                    fd.as_raw_fd(),
                    self.buffer.as_slice(),
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

        if len != self.buffer.len() {
            return Err(TransportError::RawSocketWrite(format!(
                "Could not write the whole packet. Packet len: {}. Actually written: {}",
                self.buffer.len(),
                len
            )))?;
        }

        Ok(())
    }

    fn create_new_box(&self) -> Box<dyn TcpPacketWriter> {
        // fd is shared. buffer is allocated each time we clone the writer
        Box::new(AsyncFdPacketWriter::new(self.fd.clone()))
    }
}
