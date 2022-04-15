use super::{Buffer, Checked, Code, Protocol};
use crate::Error;
use alloc::borrow::Cow;
use core::fmt;
use core::str::FromStr;
use unsigned_varint::encode;

#[cfg(feature = "std")]
impl Protocol<'_> for std::net::Ipv4Addr {
    const CODE: Code = Code::new(4);
    const PREFIX: &'static str = "ip4";

    fn read_str(input: Checked<&str>) -> Result<Self, Error> {
        std::net::Ipv4Addr::from_str(&input).map_err(|e| Error::custom(e.into()))
    }

    fn read_bytes(input: Checked<&[u8]>) -> Result<Self, Error> {
        let mut b = [0; 4];
        b.copy_from_slice(&input);
        Ok(std::net::Ipv4Addr::from(b))
    }

    fn write_str(&self, f: &mut fmt::Formatter) -> Result<(), Error> {
        write!(f, "/{}/{}", Self::PREFIX, self)?;
        Ok(())
    }

    fn write_bytes(&self, buf: &mut dyn Buffer) {
        let mut b = encode::u32_buffer();
        let uvi = encode::u32(Self::CODE.into(), &mut b);
        buf.extend_with(uvi);
        buf.extend_with(&self.octets())
    }
}

#[cfg(feature = "std")]
impl Protocol<'_> for std::net::Ipv6Addr {
    const CODE: Code = Code::new(41);
    const PREFIX: &'static str = "ip6";

    fn read_str(input: Checked<&str>) -> Result<Self, Error> {
        std::net::Ipv6Addr::from_str(&input).map_err(|e| Error::custom(e.into()))
    }

    fn read_bytes(input: Checked<&[u8]>) -> Result<Self, Error> {
        let mut b = [0; 16];
        b.copy_from_slice(&input);
        Ok(std::net::Ipv6Addr::from(b))
    }

    fn write_str(&self, f: &mut fmt::Formatter) -> Result<(), Error> {
        write!(f, "/{}/{}", Self::PREFIX, self)?;
        Ok(())
    }

    fn write_bytes(&self, buf: &mut dyn Buffer) {
        let mut b = encode::u32_buffer();
        let uvi = encode::u32(Self::CODE.into(), &mut b);
        buf.extend_with(uvi);
        buf.extend_with(&self.octets())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tcp(pub u16);

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dns<'a>(pub Cow<'a, str>);

impl<'a> Dns<'a> {
    pub fn new<S: Into<Cow<'a, str>>>(s: S) -> Self {
        Dns(s.into())
    }
}

impl<'a> Protocol<'a> for Dns<'a> {
    const CODE: Code = Code::new(53);
    const PREFIX: &'static str = "dns";

    fn read_str(input: Checked<&'a str>) -> Result<Self, Error> {
        Ok(Dns(Cow::Borrowed(input.0)))
    }

    fn read_bytes(input: Checked<&'a [u8]>) -> Result<Self, Error> {
        let s = core::str::from_utf8(&input).map_err(Error::message)?;
        Ok(Dns(Cow::Borrowed(s)))
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
