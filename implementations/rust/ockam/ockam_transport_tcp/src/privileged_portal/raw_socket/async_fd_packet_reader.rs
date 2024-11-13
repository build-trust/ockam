use crate::privileged_portal::packet::{
    Ipv4Info, RawSocketReadResult, TcpInfo, TcpStrippedHeaderAndPayload,
};
use crate::privileged_portal::packet_binary::{ipv4_header, tcp_header};
use crate::privileged_portal::TcpPacketReader;
use async_trait::async_trait;
use log::{error, trace};
use nix::sys::socket::MsgFlags;
use ockam_core::Result;
use ockam_transport_core::TransportError;
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::Arc;
use tokio::io::unix::AsyncFd;
use tokio::io::Interest;

/// RawSocket packet reader implemented via tokio's AsyncFd
pub struct AsyncFdPacketReader {
    buffer: Vec<u8>,
    fd: Arc<AsyncFd<OwnedFd>>,
}

struct ParseHeadersRes {
    ipv4_info: Ipv4Info,
    tcp_info: TcpInfo,
    // Offset at which the stripped part of the TCP Header (ports are stripped) and payload starts
    offset: usize,
}

impl AsyncFdPacketReader {
    /// Constructor
    pub fn new(fd: Arc<AsyncFd<OwnedFd>>) -> Self {
        Self {
            buffer: vec![0; 65535],
            fd,
        }
    }

    fn parse_headers(buffer: &mut [u8]) -> Result<ParseHeadersRes> {
        if Ipv4Info::HEADER_BASE_LEN > buffer.len() {
            return Err(TransportError::ParsingHeaders(
                "Buffer can't fit IPv4 header".to_string(),
            ))?;
        }

        let mut offset = 0;

        let ipv4_view = ipv4_header::View::new(&buffer[offset..offset + Ipv4Info::HEADER_BASE_LEN]);
        let ipv4_info = Ipv4Info::from(&ipv4_view);

        if buffer.len() != ipv4_info.total_length() as usize {
            return Err(TransportError::ParsingHeaders(format!(
                "IPv4 packet length doesn't match buffer length. IPv4: {} buffer: {}",
                ipv4_info.total_length(),
                buffer.len(),
            )))?;
        }

        offset += ipv4_info.header_length() as usize;

        if offset + TcpInfo::HEADER_BASE_LEN > buffer.len() {
            return Err(TransportError::ParsingHeaders(
                "Buffer can't fit TCP header".to_string(),
            ))?;
        }

        let mut tcp_view =
            tcp_header::View::new(&mut buffer[offset..offset + TcpInfo::HEADER_BASE_LEN]);

        // Reset the checksum
        tcp_view.check_mut().write(0);

        let tcp_info = TcpInfo::from(&tcp_view);

        // Only skip ports, everything else we want in that binary
        offset += 4;

        // TODO: Add some sanity checks? Checksum?

        trace!(
            "TCP: {}:{} -> {}:{}. IPv4 length: header-{} total-{}. TCP header length: {}. Flags: {}",
            ipv4_info.source_ip(),
            tcp_info.source_port(),
            ipv4_info.destination_ip(),
            tcp_info.destination_port(),
            ipv4_info.header_length(),
            ipv4_info.total_length(),
            tcp_info.header_length(),
            tcp_info.flags(),
        );

        Ok(ParseHeadersRes {
            ipv4_info,
            tcp_info,
            offset,
        })
    }
}

#[async_trait]
impl TcpPacketReader for AsyncFdPacketReader {
    async fn read_packet(&mut self) -> Result<RawSocketReadResult> {
        enum ReadResult {
            Ok { len: usize },
            Err(TransportError),
        }

        let read_result = self
            .fd
            .async_io(Interest::READABLE, |fd| {
                let res =
                    nix::sys::socket::recv(fd.as_raw_fd(), &mut self.buffer, MsgFlags::empty());

                match res {
                    Ok(len) => Ok(ReadResult::Ok { len }),
                    Err(err) => {
                        if err == nix::errno::Errno::EWOULDBLOCK {
                            trace!("RawSocket read would block");
                            return Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, err));
                        }
                        error!("Failed to receive packet. Err: {}", err);
                        Ok(ReadResult::Err(TransportError::RawSocketRead(
                            err.to_string(),
                        )))
                    }
                }
            })
            .await
            .map_err(|err| TransportError::RawSocketRead(err.to_string()))?;

        let len = match read_result {
            ReadResult::Ok { len } => len,
            ReadResult::Err(err) => return Err(err)?,
        };

        // This code doesn't account for the case where packets can be split, which should
        // not really happen, since even with custom proto value, they're still valid IPv4
        // packets, that Kernel should not split

        let ParseHeadersRes {
            ipv4_info,
            tcp_info,
            offset,
        } = Self::parse_headers(&mut self.buffer[..len])?;

        let header_and_payload =
            match TcpStrippedHeaderAndPayload::new(self.buffer[offset..len].to_vec().into()) {
                Some(header_and_payload) => header_and_payload,
                None => {
                    return Err(TransportError::ParsingHeaders(
                        "Packet stripped header is too short".to_string(),
                    ))?
                }
            };

        let res = RawSocketReadResult {
            ipv4_info,
            tcp_info,
            header_and_payload,
        };

        Ok(res)
    }
}
