use crate::privileged_portal::{
    AsyncFdPacketReader, AsyncFdPacketWriter, Proto, TcpPacketReader, TcpPacketWriter,
};
use nix::errno::Errno;
use nix::sys::socket::{AddressFamily, SockFlag, SockProtocol, SockType};
use ockam_core::Result;
use ockam_transport_core::TransportError;
use std::mem;
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::Arc;
use tokio::io::unix::AsyncFd;

/// Create RawSocket and instantiate TcpPacketWriter and TcpPacketReader implemented via tokio's
/// AsyncFd
pub fn create_async_fd_raw_socket(
    proto: Proto,
) -> Result<(Box<dyn TcpPacketWriter>, Box<dyn TcpPacketReader>)> {
    let fd = create_raw_socket_fd(proto)?;
    let fd = Arc::new(fd);

    let writer = AsyncFdPacketWriter::new(fd.clone());
    let writer = Box::new(writer);

    let reader = AsyncFdPacketReader::new(fd);
    let reader = Box::new(reader);

    Ok((writer, reader))
}

fn create_raw_socket_fd(proto: Proto) -> Result<AsyncFd<OwnedFd>> {
    // Unfortunately, SockProtocol enum doesn't support arbitrary values
    let proto: SockProtocol = unsafe { mem::transmute(proto as i32) };
    let res = nix::sys::socket::socket(
        AddressFamily::Inet,
        SockType::Raw,
        SockFlag::SOCK_NONBLOCK,
        Some(proto),
    );

    let socket = match res {
        Ok(socket) => socket,
        Err(err) => {
            return Err(TransportError::RawSocketCreation(err.to_string()))?;
        }
    };

    // TODO: It's possible to bind that socket to an IP if needed

    let res = unsafe {
        // We don't want to construct IPv4 header ourselves, for receiving it will be included
        // nevertheless
        let hincl: nix::libc::c_int = 0;
        #[allow(trivial_casts)]
        nix::libc::setsockopt(
            socket.as_raw_fd(),
            nix::libc::IPPROTO_IP,
            nix::libc::IP_HDRINCL,
            (&hincl as *const nix::libc::c_int) as *const nix::libc::c_void,
            size_of::<nix::libc::c_int>() as nix::libc::socklen_t,
        )
    };

    if res == -1 {
        Err(TransportError::RawSocketCreation(
            Errno::last_raw().to_string(),
        ))?;
    }

    let async_fd =
        AsyncFd::new(socket).map_err(|e| TransportError::RawSocketCreation(e.to_string()))?;

    Ok(async_fd)
}
