use std::{io, net::SocketAddr, os::fd::AsRawFd, ptr};

use socket2::{SockAddr, Socket, Type};
use sysctl::Sysctl;

/// MPTCP Socket builder
#[derive(Debug)]
pub struct MptcpSocketBuilder(Socket);

impl MptcpSocketBuilder {
    /// Constructor
    pub fn new() -> io::Result<Self> {
        const AF_MULTIPATH: nix::libc::c_int = 39;

        Ok(Self(Socket::new(AF_MULTIPATH.into(), Type::STREAM, None)?))
    }

    /// Constructor (has unused parameter to have the same API as on Linux)
    pub fn new_for_addr(_addr: SocketAddr) -> io::Result<Self> {
        Self::new()
    }

    /// Set nonblocking
    pub fn set_nonblocking(self) -> io::Result<Self> {
        self.0.set_nonblocking(true)?;
        Ok(self)
    }

    /// Set REUSE_ADDR
    pub fn set_reuse(self) -> io::Result<Self> {
        self.0.reuse_address()?;
        Ok(self)
    }

    /// Connect
    pub fn connect(self, addr: SocketAddr) -> io::Result<Socket> {
        let socket = self.0;
        let addr: &SockAddr = &addr.into();

        let sae = nix::libc::sa_endpoints_t {
            sae_srcif: 0,
            sae_srcaddr: ptr::null(),
            sae_srcaddrlen: 0,
            sae_dstaddr: addr.as_ptr(),
            sae_dstaddrlen: addr.len(),
        };

        let ret = match unsafe {
            nix::libc::connectx(
                socket.as_raw_fd(),
                &sae,
                nix::libc::SAE_ASSOCID_ANY,
                0,
                ptr::null(),
                0,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        } {
            -1 => Err(io::Error::last_os_error()),
            _ => Ok(()),
        };

        match ret {
            Err(err) if err.raw_os_error() != Some(nix::libc::EINPROGRESS) => Err(err),
            _ => Ok(socket),
        }
    }

    /// Bind
    pub fn bind(self, _addr: SocketAddr) -> io::Result<Socket> {
        // bind is not supported for AF_MULTIPATH sockets
        Err(io::ErrorKind::Unsupported.into())
    }
}

/// Check if MPTCP support is enabled
pub fn is_mptcp_enabled() -> bool {
    if let Ok(ctl) = sysctl::Ctl::new("net.inet.mptcp.enable") {
        if let Ok(val) = ctl.value() {
            if let Some(val) = val.as_string() {
                return val == "1";
            }
        }
    }

    false
}
