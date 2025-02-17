use std::{io, net::SocketAddr};

use semver::Version;
use socket2::{Domain, Protocol, Socket, Type};
use sysctl::Sysctl;
use sysinfo::System;

lazy_static::lazy_static! {
    static ref KERNEL_VERSION : Option<Version> = System::kernel_version().and_then(|v| Version::parse(&v).ok());
}

/// MPTCP Socket builder
#[derive(Debug)]
pub struct MptcpSocketBuilder(Socket);

impl MptcpSocketBuilder {
    /// Constructor
    fn new(domain: Domain) -> io::Result<Self> {
        Ok(Self(Socket::new(
            domain,
            Type::STREAM,
            Some(Protocol::MPTCP),
        )?))
    }

    /// Constructor
    pub fn new_for_addr(addr: SocketAddr) -> io::Result<Self> {
        Self::new(Domain::for_address(addr))
    }

    /// Set nonblocking
    pub fn set_nonblocking(self) -> io::Result<Self> {
        self.0.set_nonblocking(true)?;
        Ok(self)
    }

    /// Connect
    pub fn connect(self, addr: SocketAddr) -> io::Result<Socket> {
        let socket = self.0;

        match socket
            .connect(&addr.into())
            .map_err(|e| (e.raw_os_error(), e))
        {
            Err((Some(errno), err)) if errno != nix::libc::EINPROGRESS => Err(err),
            _ => Ok(socket),
        }
    }

    /// Bind
    pub fn bind(self, addr: SocketAddr) -> io::Result<Socket> {
        let socket = self.0;
        socket.bind(&addr.into())?;
        socket.listen(0)?;
        Ok(socket)
    }
}

/// Check if MPTCP support is enabled
pub fn is_mptcp_enabled() -> bool {
    if let Ok(ctl) = sysctl::Ctl::new("net.mptcp.enabled") {
        if let Ok(val) = ctl.value() {
            if let Some(val) = val.as_string() {
                return val == "1";
            }
        }
    }

    false
}
