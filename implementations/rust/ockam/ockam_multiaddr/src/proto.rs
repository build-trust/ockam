use super::{Buffer, Checked, Code, Protocol};
use crate::Error;
use alloc::borrow::Cow;
use core::fmt;
use core::ops::Deref;
use core::str::{self, FromStr};
use unsigned_varint::encode;

/// An IPv4 address.
#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ip4(pub std::net::Ipv4Addr);

#[cfg(feature = "std")]
impl Ip4 {
    pub fn new<T: Into<std::net::Ipv4Addr>>(v: T) -> Self {
        Ip4(v.into())
    }
}

#[cfg(feature = "std")]
impl Deref for Ip4 {
    type Target = std::net::Ipv4Addr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "std")]
impl Protocol<'_> for Ip4 {
    const CODE: Code = Code::new(4);
    const PREFIX: &'static str = "ip4";

    fn read_str(input: Checked<&str>) -> Result<Self, Error> {
        std::net::Ipv4Addr::from_str(&input)
            .map(Ip4)
            .map_err(|e| Error::custom(e.into()))
    }

    fn read_bytes(input: Checked<&[u8]>) -> Result<Self, Error> {
        let mut b = [0; 4];
        b.copy_from_slice(&input);
        Ok(Ip4(std::net::Ipv4Addr::from(b)))
    }

    fn write_str(&self, f: &mut fmt::Formatter) -> Result<(), Error> {
        write!(f, "/{}/{}", Self::PREFIX, self.0)?;
        Ok(())
    }

    fn write_bytes(&self, buf: &mut dyn Buffer) {
        let mut b = encode::u32_buffer();
        let uvi = encode::u32(Self::CODE.into(), &mut b);
        buf.extend_with(uvi);
        buf.extend_with(&self.0.octets())
    }
}

/// An IPv6 address.
#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ip6(pub std::net::Ipv6Addr);

#[cfg(feature = "std")]
impl Ip6 {
    pub fn new<T: Into<std::net::Ipv6Addr>>(v: T) -> Self {
        Ip6(v.into())
    }
}

#[cfg(feature = "std")]
impl Deref for Ip6 {
    type Target = std::net::Ipv6Addr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "std")]
impl Protocol<'_> for Ip6 {
    const CODE: Code = Code::new(41);
    const PREFIX: &'static str = "ip6";

    fn read_str(input: Checked<&str>) -> Result<Self, Error> {
        std::net::Ipv6Addr::from_str(&input)
            .map(Ip6)
            .map_err(|e| Error::custom(e.into()))
    }

    fn read_bytes(input: Checked<&[u8]>) -> Result<Self, Error> {
        let mut b = [0; 16];
        b.copy_from_slice(&input);
        Ok(Ip6(std::net::Ipv6Addr::from(b)))
    }

    fn write_str(&self, f: &mut fmt::Formatter) -> Result<(), Error> {
        write!(f, "/{}/{}", Self::PREFIX, self.0)?;
        Ok(())
    }

    fn write_bytes(&self, buf: &mut dyn Buffer) {
        let mut b = encode::u32_buffer();
        let uvi = encode::u32(Self::CODE.into(), &mut b);
        buf.extend_with(uvi);
        buf.extend_with(&self.0.octets())
    }
}

/// A TCP port number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tcp(pub u16);

impl Tcp {
    pub fn new(v: u16) -> Self {
        Tcp(v)
    }
}

impl Deref for Tcp {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Protocol<'_> for Tcp {
    const CODE: Code = Code::new(6);
    const PREFIX: &'static str = "tcp";

    fn read_str(input: Checked<&str>) -> Result<Self, Error> {
        u16::from_str(&input).map(Tcp).map_err(Error::message)
    }

    fn read_bytes(input: Checked<&[u8]>) -> Result<Self, Error> {
        let mut b = [0; 2];
        b.copy_from_slice(&input);
        Ok(Tcp(u16::from_be_bytes(b)))
    }

    fn write_str(&self, f: &mut fmt::Formatter) -> Result<(), Error> {
        write!(f, "/{}/{}", Self::PREFIX, self.0)?;
        Ok(())
    }

    fn write_bytes(&self, buf: &mut dyn Buffer) {
        let mut b = encode::u32_buffer();
        let uvi = encode::u32(Self::CODE.into(), &mut b);
        buf.extend_with(uvi);
        buf.extend_with(&self.0.to_be_bytes())
    }
}

/// A Udp port number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Udp(pub u16);

impl Udp {
    pub fn new(v: u16) -> Self {
        Udp(v)
    }
}

impl Deref for Udp {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Protocol<'_> for Udp {
    const CODE: Code = Code::new(273);
    const PREFIX: &'static str = "udp";

    fn read_str(input: Checked<&str>) -> Result<Self, Error> {
        u16::from_str(&input).map(Udp).map_err(Error::message)
    }

    fn read_bytes(input: Checked<&[u8]>) -> Result<Self, Error> {
        let mut b = [0; 2];
        b.copy_from_slice(&input);
        Ok(Udp(u16::from_be_bytes(b)))
    }

    fn write_str(&self, f: &mut fmt::Formatter) -> Result<(), Error> {
        write!(f, "/{}/{}", Self::PREFIX, self.0)?;
        Ok(())
    }

    fn write_bytes(&self, buf: &mut dyn Buffer) {
        let mut b = encode::u32_buffer();
        let uvi = encode::u32(Self::CODE.into(), &mut b);
        buf.extend_with(uvi);
        buf.extend_with(&self.0.to_be_bytes())
    }
}

macro_rules! gen_str_proto {
    ($t:ident, $c:literal, $p:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $t<'a>(Cow<'a, str>);

        impl<'a> $t<'a> {
            pub fn new<S: Into<Cow<'a, str>>>(s: S) -> Self {
                Self(s.into())
            }
        }

        impl Deref for $t<'_> {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl<'a> Protocol<'a> for $t<'a> {
            const CODE: Code = Code::new($c);
            const PREFIX: &'static str = $p;

            fn read_str(input: Checked<&'a str>) -> Result<Self, Error> {
                Ok(Self(Cow::Borrowed(input.0)))
            }

            fn read_bytes(input: Checked<&'a [u8]>) -> Result<Self, Error> {
                let s = str::from_utf8(&input).map_err(Error::message)?;
                Ok(Self(Cow::Borrowed(s)))
            }

            fn write_str(&self, f: &mut fmt::Formatter) -> Result<(), Error> {
                write!(f, "/{}/{}", Self::PREFIX, self.0)?;
                Ok(())
            }

            fn write_bytes(&self, buf: &mut dyn Buffer) {
                let mut b = encode::u32_buffer();
                let uvi = encode::u32(Self::CODE.into(), &mut b);
                buf.extend_with(uvi);
                let mut b = encode::usize_buffer();
                let uvi = encode::usize(self.0.len(), &mut b);
                buf.extend_with(uvi);
                buf.extend_with(self.0.as_bytes())
            }
        }
    };
}

gen_str_proto!(Worker, 102526, "worker");
gen_str_proto!(DnsAddr, 56, "dnsaddr");
gen_str_proto!(Service, 62526, "service");
gen_str_proto!(Node, 72526, "node");
gen_str_proto!(Project, 82526, "project");
gen_str_proto!(Space, 92526, "space");
gen_str_proto!(Secure, 99526, "secure");
